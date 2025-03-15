pub struct AudioPlaybackThread {
    control_sender: smol::channel::Sender<i64>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioPlaybackThread {
    fn dbg(codec: &Codec) {
        println!("type: decoder");
        println!("\t id: {:?}", codec.id());
        println!("\t name: {}", codec.name());
        println!("\t description: {}", codec.description());
        println!("\t medium: {:?}", codec.medium());
        println!("\t capabilities: {:?}", codec.capabilities());

        if let Some(profiles) = codec.profiles() {
            println!("\t profiles: {:?}", profiles.collect::<Vec<_>>());
        } else {
            println!("\t profiles: none");
        }

        if let Ok(video) = codec.video() {
            if let Some(rates) = video.rates() {
                println!("\t rates: {:?}", rates.collect::<Vec<_>>());
            } else {
                println!("\t rates: any");
            }

            if let Some(formats) = video.formats() {
                println!("\t formats: {:?}", formats.collect::<Vec<_>>());
            } else {
                println!("\t formats: any");
            }
        }

        if let Ok(audio) = codec.audio() {
            if let Some(rates) = audio.rates() {
                println!("\t rates: {:?}", rates.collect::<Vec<_>>());
            } else {
                println!("\t rates: any");
            }

            if let Some(formats) = audio.formats() {
                println!("\t formats: {:?}", formats.collect::<Vec<_>>());
            } else {
                println!("\t formats: any");
            }

            if let Some(layouts) = audio.channel_layouts() {
                println!("\t channel_layouts: {:?}", layouts.collect::<Vec<_>>());
            } else {
                println!("\t channel_layouts: any");
            }
        }

        println!("\t max_lowres: {:?}", codec.max_lowres());
    }
    pub fn start(is_master:bool,stream: &ffmpeg_next::format::stream::Stream, sender: smol::channel::Sender<(i64,String)>) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::bounded(128);
        let (packet_sender, packet_receiver) = smol::channel::bounded(30);
        let mut decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        let mut packet_decoder = decoder_context.decoder().audio()?;
        packet_decoder.set_parameters(stream.parameters())?;
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device available");
        let config = device.default_output_config()?;
        if packet_decoder.channel_layout().is_empty() {
            packet_decoder.set_channel_layout(ffmpeg::ChannelLayout::default(2));
        }
        
        let sender2 = sender.clone();
        let mut clock = StreamClock::new(stream,1.0);
        let receiver_thread =
            std::thread::Builder::new().name("audio playback thread".into()).spawn(move || {
                smol::block_on(async move {
                    let output_channel_layout = match config.channels() {
                        1 => ffmpeg_next::util::channel_layout::ChannelLayout::MONO,
                        _ => ffmpeg_next::util::channel_layout::ChannelLayout::STEREO,
                        // _ => todo!(),
                    };

                    let mut ffmpeg_to_cpal_forwarder = match config.sample_format() {
                        cpal::SampleFormat::U8 => FFmpegToCPalForwarder::new::<u8>(
                            config,
                            &device,
                            control_receiver,
                            packet_receiver,
                            packet_decoder,
                            ffmpeg_next::util::format::sample::Sample::U8(
                                ffmpeg_next::util::format::sample::Type::Packed,
                            ),
                            output_channel_layout,
                            clock
                        ),
                        cpal::SampleFormat::F32 => FFmpegToCPalForwarder::new::<f32>(
                            config,
                            &device,
                            control_receiver,
                            packet_receiver,
                            packet_decoder,
                            ffmpeg_next::util::format::sample::Sample::F32(
                                ffmpeg_next::util::format::sample::Type::Packed
                            ),
                            output_channel_layout,
                            clock
                        ),
                        format @ _ => todo!("unsupported cpal output format {:#?}", format),
                    };

                    let packet_receiver_impl =
                        async {
                            ffmpeg_to_cpal_forwarder.stream().await;
                            ffmpeg_to_cpal_forwarder.silent
                        }.fuse().shared();

                    let mut playing = true;

                    loop {
                        let packet_receiver: OptionFuture<_> =
                            if playing { Some(packet_receiver_impl.clone()) } else { None }.into();

                        smol::pin!(packet_receiver);

                        futures::select! {
                            silent = packet_receiver => {
                                println!("Audio silence: {}",silent.unwrap());
                                if is_master && !silent.unwrap() { sender.send((Media::EOF,"".to_string())).await; }
                                break;
                               
                            }
                        }
                    }
                });
                sender2.send_blocking((Player::AUDIO_DEAD,"".to_string())).unwrap_or_default();
            })?;

        Ok(Self { control_sender, packet_sender, receiver_thread: Some(receiver_thread) })
    }

    pub async fn receive_packet(&self, packet: ffmpeg_next::codec::packet::packet::Packet) -> bool {
        match self.packet_sender.send(packet).await {
            Ok(_) => return true,
            Err(smol::channel::SendError(_)) => return false,
        }
    }

    pub async fn send_control_message(&self, message: i64) {
        self.control_sender.send(message).await.unwrap_or(());
    }


    pub fn flush_to_end(&self) {
        while !self.packet_sender.is_empty() {
            thread::sleep(Duration::from_nanos(1));
        }
    }
}

impl Drop for AudioPlaybackThread {
    fn drop(&mut self) {
        //      println!("DROPPING AUDIO");
        self.send_control_message(Player::CTL_AUDIO_DIE);
        //  self.flush_to_end();
        //    self.control_sender.close();
        if let Some(receiver_join_handle) = self.receiver_thread.take() {
            receiver_join_handle.join().unwrap_or(());
        }
    }
}

trait FFMpegToCPalSampleForwarder {
    fn forward(
        &mut self,
        audio_frame: ffmpeg_next::frame::Audio,
    ) -> Pin<Box<dyn Future<Output = ()> + '_>>;
}

impl<T: Pod, R: RbRef> FFMpegToCPalSampleForwarder for ringbuf::Producer<T, R>
where
    <R as RbRef>::Rb: RbWrite<T>,
{
    fn forward(
        &mut self,
        audio_frame: ffmpeg_next::frame::Audio,
    ) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(async move {
            let mut expected_bytes = 
                audio_frame.samples() * audio_frame.channels() as usize * core::mem::size_of::<T>();
            if expected_bytes > audio_frame.data(0).len() { expected_bytes = audio_frame.data(0).len() };
            let sd: &[T] = bytemuck::cast_slice(&audio_frame.data(0)[..expected_bytes]);

            while self.free_len() < sd.len() {
                smol::Timer::after(std::time::Duration::from_millis(16)).await;
            }
            self.push_slice(sd);
        })
    }
}

struct FFmpegToCPalForwarder {
    silent: bool,
    _cpal_stream: cpal::Stream,
    ffmpeg_to_cpal_pipe: Box<dyn FFMpegToCPalSampleForwarder>,
    control_receiver: smol::channel::Receiver<i64>,
    packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
    packet_decoder: ffmpeg_next::decoder::Audio,
    resampler: ffmpeg_next::software::resampling::Context,
    clock: StreamClock
}

impl FFmpegToCPalForwarder {
    fn new<T: Send + Pod + SizedSample + 'static>(
        config: cpal::SupportedStreamConfig,
        device: &cpal::Device,
        control_receiver: smol::channel::Receiver<i64>,
        packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
        mut packet_decoder: ffmpeg_next::decoder::Audio,
        output_format: ffmpeg_next::util::format::sample::Sample,
        output_channel_layout: ffmpeg_next::util::channel_layout::ChannelLayout,
        clock: StreamClock
    ) -> Self {
        let buffer = HeapRb::new(1024*32);
        let (sample_producer, mut sample_consumer) = buffer.split();

        let cpal_stream = device
            .build_output_stream(
                &config.config(),
                move |data, _| {
                    let filled = sample_consumer.pop_slice(data);
                    data[filled..].fill(T::EQUILIBRIUM);
                },
                move |err| {
                    eprintln!("error feeding audio stream to cpal: {}", err);
                },
                None,
            )
            .unwrap();
        
        cpal_stream.play().unwrap();
        
        let resampler = u!(ffmpeg_next::software::resampling::Context::get(
            packet_decoder.format(),
            packet_decoder.channel_layout(),
            packet_decoder.rate(),
            output_format,
            output_channel_layout,
            config.sample_rate().0,
        ));
        
        Self {
            silent: false,
            _cpal_stream: cpal_stream,
            ffmpeg_to_cpal_pipe: Box::new(sample_producer),
            control_receiver,
            packet_receiver,
            packet_decoder,
            resampler,
            clock
        }
    }

    async fn stream(&mut self) {
        let mut rsc = 1000;
        let mut origin = 0;
        
        loop {
            rsc-=1;
            if rsc<0 { rsc = 1000 }

            // Receive the next packet from the packet receiver channel.
            let Ok(packet) = self.packet_receiver.recv().await else { break };

            // Send the packet to the decoder.
            self.packet_decoder.send_packet(&packet).unwrap();
            // Create an empty frame to hold the decoded audio data.
            let mut decoded_frame = ffmpeg_next::util::frame::Audio::empty();
            // Continue receiving decoded frames until there are no more available.
            let mut shot = false;

            while self.packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                shot = true;
                futures::select! {
                    _ = futures::FutureExt::fuse(smol::Timer::after(std::time::Duration::from_millis(8))) => {}
                    cmd = self.control_receiver.recv().fuse() => {
                        match cmd{
                            Ok(Player::CTL_AUDIO_DIE_SILENT)=> {
                                self.silent = true;
                                return;
                            }
                            Ok(Player::CTL_AUDIO_DIE)=> {
                       //         println!("KILLING AUDIO");
                                return;
                            }
                            Ok(Player::CTL_SEEK_ABS)|Ok(Player::CTL_SEEK_REL) => {
                                while !self.packet_receiver.is_empty() { self.packet_receiver.recv_blocking().unwrap(); }
                         //       println!("AUDIO SKIPPED");
                            }
                            _=> {}
                        }
                    }
                }
                let mut resampled_frame = ffmpeg_next::util::frame::Audio::empty();
                self.resampler.run(&decoded_frame, &mut resampled_frame).unwrap_or_default();
                if !resampled_frame.is_corrupt() {
                    let pts = decoded_frame.pts();
                    if self.clock.start_time.is_none() {
                        origin = u!(pts);
                    }
                    if let Some(mut delay) =
                        self.clock.convert_pts_to_instant(pts,origin)
                    {
                        let delay_ms: u64 = delay.as_millis() as u64;
                        if delay_ms>=0 && delay_ms<50  {
                            delay = Duration::from_millis(delay_ms);
                            let dof = Duration::from_secs_f64(0.011);
                            delay = delay.checked_sub(dof).unwrap_or(dof);
                            self.clock.start_time = Option::from(self.clock.start_time.unwrap().checked_sub(dof).unwrap_or(self.clock.start_time.unwrap()));
                            smol::Timer::after(delay).await;
                        }
                    }
                    self.ffmpeg_to_cpal_pipe.forward(resampled_frame).await; 
                }
            }
            if !shot {
                futures::select! {
                    _ = futures::FutureExt::fuse(smol::Timer::after(std::time::Duration::from_millis(4))) => {}
                    cmd = self.control_receiver.recv().fuse() => {
                        match cmd{
                            Ok(Player::CTL_AUDIO_DIE)=> {
                //                println!("KILLING AUDIO");
                                return;
                            }
                            Ok(Player::CTL_SEEK_ABS)|Ok(Player::CTL_SEEK_REL) => {
                                while !self.packet_receiver.is_empty() { self.packet_receiver.recv_blocking().unwrap(); }
                                self.clock.start_time = Option::None;
                  //              println!("AUDIO SKIPPED");
                            }
                            _=> {}
                        }
                    }
                }
            }
        }
    }
}
