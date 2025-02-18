use std::pin::Pin;

use bytemuck::Pod;
use cpal::SizedSample;

use futures::future::OptionFuture;
use futures::FutureExt;
use ringbuf::ring_buffer::RbRef;
use ringbuf::ring_buffer::RbWrite;
use ringbuf::HeapRb;
use std::future::Future;

pub struct AudioPlaybackThread {
    control_sender: smol::channel::Sender<i64>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioPlaybackThread {
    pub fn start(stream: &ffmpeg_next::format::stream::Stream, sender: smol::channel::Sender<i32>) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::bounded(128);

        let (packet_sender, packet_receiver) = smol::channel::bounded(128);

        let decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        let packet_decoder = decoder_context.decoder().audio()?;

        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device available");

        let config = device.default_output_config().unwrap();

        let receiver_thread =
            std::thread::Builder::new().name("audio playback thread".into()).spawn(move || {
                smol::block_on(async move {
                    let output_channel_layout = match config.channels() {
                        1 => ffmpeg_next::util::channel_layout::ChannelLayout::MONO,
                        2 => ffmpeg_next::util::channel_layout::ChannelLayout::STEREO,
                        _ => todo!(),
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
                        ),
                        cpal::SampleFormat::F32 => FFmpegToCPalForwarder::new::<f32>(
                            config,
                            &device,
                            control_receiver,
                            packet_receiver,
                            packet_decoder,
                            ffmpeg_next::util::format::sample::Sample::F32(
                                ffmpeg_next::util::format::sample::Type::Packed,
                            ),
                            output_channel_layout,
                        ),
                        format @ _ => todo!("unsupported cpal output format {:#?}", format),
                    };

                    let packet_receiver_impl =
                        async { ffmpeg_to_cpal_forwarder.stream().await }.fuse().shared();

                    let mut playing = true;

                    loop {
                        let packet_receiver: OptionFuture<_> =
                            if playing { Some(packet_receiver_impl.clone()) } else { None }.into();

                        smol::pin!(packet_receiver);

                        futures::select! {
                            _ = packet_receiver => {},
                            /*received_command = control_receiver.recv().fuse() => {
                                match received_command {
                                    Ok(ControlCommand::Die) => {
                                        return;
                                    }
                                    Ok(ControlCommand::Play) => {
                                        playing = true;
                                    }
                                    _ => {}
                                }
                            }*/
                        }
                    }
                });
                sender.send_blocking(Media::EOF).unwrap_or_default();
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
        self.control_sender.send(message).await.unwrap();
    }

    pub fn flush_to_end(&self) {
        while !self.packet_sender.is_empty() {
            thread::sleep(Duration::from_nanos(1));
        }
    }
}

impl Drop for AudioPlaybackThread {
    fn drop(&mut self) {
        self.flush_to_end();
        self.control_sender.close();
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
            // Audio::plane() returns the wrong slice size, so correct it by hand. See also
            // for a fix https://github.com/zmwangx/rust-ffmpeg/pull/104.
            let expected_bytes =
                audio_frame.samples() * audio_frame.channels() as usize * core::mem::size_of::<T>();
            let cpal_sample_data: &[T] =
                bytemuck::cast_slice(&audio_frame.data(0)[..expected_bytes]);

            while self.free_len() < cpal_sample_data.len() {
                smol::Timer::after(std::time::Duration::from_millis(16)).await;
            }

            // Buffer the samples for playback
            self.push_slice(cpal_sample_data);
        })
    }
}

struct FFmpegToCPalForwarder {
    _cpal_stream: cpal::Stream,
    ffmpeg_to_cpal_pipe: Box<dyn FFMpegToCPalSampleForwarder>,
    control_receiver: smol::channel::Receiver<i64>,
    packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
    packet_decoder: ffmpeg_next::decoder::Audio,
    resampler: ffmpeg_next::software::resampling::Context,
}



impl FFmpegToCPalForwarder {
    fn new<T: Send + Pod + SizedSample + 'static>(
        config: cpal::SupportedStreamConfig,
        device: &cpal::Device,
        control_receiver: smol::channel::Receiver<i64>,
        packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
        packet_decoder: ffmpeg_next::decoder::Audio,
        output_format: ffmpeg_next::util::format::sample::Sample,
        output_channel_layout: ffmpeg_next::util::channel_layout::ChannelLayout,
    ) -> Self {
        let buffer = HeapRb::new(1024*16);
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

        let resampler = ffmpeg_next::software::resampling::Context::get(
            packet_decoder.format(),
            packet_decoder.channel_layout(),
            packet_decoder.rate(),
            output_format,
            output_channel_layout,
            config.sample_rate().0,
        )
        .unwrap();

        Self {
            _cpal_stream: cpal_stream,
            ffmpeg_to_cpal_pipe: Box::new(sample_producer),
            control_receiver,
            packet_receiver,
            packet_decoder,
            resampler,
        }
    }

    async fn stream(&mut self) {
        loop {
            if !self.control_receiver.is_empty() {
                //  println!("RECV");
                match self.control_receiver.recv_blocking() {
                    Ok(Player::CTL_DIE)=> {
                        println!("KILLING AUDIO");
                        return;
                    }
                    Ok(Player::CTL_SEEK_ABS)|Ok(Player::CTL_SEEK_REL) => {
                        while !self.packet_receiver.is_empty() { self.packet_receiver.recv_blocking().unwrap(); }
                        println!("AUDIO SKIPPED");
                    }
                    _ => {}
                }
            }
            // Receive the next packet from the packet receiver channel.
            let Ok(packet) = self.packet_receiver.recv().await else { break };

            // Send the packet to the decoder.
            self.packet_decoder.send_packet(&packet).unwrap();
            // Create an empty frame to hold the decoded audio data.
            let mut decoded_frame = ffmpeg_next::util::frame::Audio::empty();
            // Continue receiving decoded frames until there are no more available.
            while self.packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                smol::Timer::after(std::time::Duration::from_millis(16)).await;
                // Create an empty frame to hold the resampled audio data.
                let mut resampled_frame = ffmpeg_next::util::frame::Audio::empty();
                self.resampler.run(&decoded_frame, &mut resampled_frame).unwrap();
                if !resampled_frame.is_corrupt() { self.ffmpeg_to_cpal_pipe.forward(resampled_frame).await; }
            }
        }
    }
}
