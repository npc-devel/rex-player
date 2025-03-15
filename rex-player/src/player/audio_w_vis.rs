use std::ffi::c_ulong;
use x11::xlib::XID;

pub struct ViAudioPlaybackThread {
    control_sender: smol::channel::Sender<i64>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl ViAudioPlaybackThread {
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
    pub fn start(im: &Visual,
                 drw: x::Drawable,
                 drb: x::Drawable,
                 bb: x::Pixmap,
                 is_master:bool,stream: &ffmpeg_next::format::stream::Stream, sender: smol::channel::Sender<(i64,String)>) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::bounded(128);

        let (packet_sender, packet_receiver) = smol::channel::bounded(30);


        let mut decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        //decoder_context.set_parameters(&ffmpeg_next::codec::Parameters::new(codec::Parameters::))?;
        
        
        let mut packet_decoder = decoder_context.decoder().audio()?;
        packet_decoder.set_parameters(stream.parameters())?;

        //let rate = u!(packet_decoder.frame_rate()).0 as f64/u!(packet_decoder.frame_rate()).1 as f64;
        //println!("Rate: {}",rate);

        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device available");

        let config = device.default_output_config()?;

        if packet_decoder.channel_layout().is_empty() {
            //let cc = self.packet_decoder.decoder().audio().unwrap();
            //packet_decoder = ffmpeg_next::codec::decoder::new().audio().expect("no new audio");

           // println!("BAD AUDIO {:?}",packet_decoder.sample_rate());
            //packet_decoder.set_parameters(stream.parameters()).expect("set audio parameters");
            //packet_decoder.set_channel_layout(ffmpeg::ChannelLayout::default(packet_decoder.channels().into()));
            //packet_decoder.set_frame_rate(Option::from(Rational(44100,1)));
            //let ic = ffmpeg_next::format::input(&"/mnt/slo/Videos/TV/Father.Ted/Series 1/5. And God Created Woman.mp4")?;
            //let a = ic.streams().best(Type::Audio).unwrap();
            //let dc = ffmpeg_next::codec::Context::from_parameters(a.parameters())?;
            //packet_decoder = dc.decoder().audio()?;
            //packet_decoder.set_parameters(stream.parameters())?;
            //packet_decoder.request_format( ffmpeg_next::util::format::sample::Sample::F32(
              //  ffmpeg_next::util::format::sample::Type::Planar
            //));
          //  packet_decoder.set_packet_time_base(1.0);
        //    packet_decoder.set_frame_rate(Option::from(Rational::new(1,44100)));
      //      packet_decoder.set_threading(threading::Config::default());
            //packet_decoder.set_flags(codec::Flags::GLOBAL_HEADER);
            packet_decoder.set_channel_layout(ffmpeg::ChannelLayout::default(2));
            //println!("TRY AUDIO");
        } //else {
            //println!("GOOD AUDIO");
        //}
        //println!("Audio packet decoder: {:?} {:?} {:?} {:?} {:?}",packet_decoder.rate(),packet_decoder.format(),packet_decoder.align(),packet_decoder.time_base(),packet_decoder.bit_rate());
      //  Self::dbg(&packet_decoder.codec().unwrap());



        //let con2 = control_sender.clone();
        //let con3 = control_sender.clone();
        let sender2 = sender.clone();
        let drv = Drawable::Window(im.window);
        let xid = im.window.resource_id() as xlib::XID;
        let m = im.clone();
        let w = m.width.clone();
        let h = m.height.clone();
        let receiver_thread =
            std::thread::Builder::new().name("audio playback thread".into()).spawn(move || {
                smol::block_on(async move {
                    let ctx = &CTX;
                    
                    //println!("Audio cfg: {:?}",config);

                    let output_channel_layout = match config.channels() {
                        1 => ffmpeg_next::util::channel_layout::ChannelLayout::MONO,
                        _ => ffmpeg_next::util::channel_layout::ChannelLayout::STEREO,
                       // _ => todo!(),
                    };

                    let mut ffmpeg_to_cpal_forwarder = match config.sample_format() {
                        /*cpal::SampleFormat::U8 => ViFFmpegToCPalForwarder::new::<u8>(
                            config,
                            &device,
                            control_receiver,
                            packet_receiver,
                            packet_decoder,
                            ffmpeg_next::util::format::sample::Sample::U8(
                                ffmpeg_next::util::format::sample::Type::Packed,
                            ),
                            output_channel_layout,
                        ),*/
                        cpal::SampleFormat::F32 => ViFFmpegToCPalForwarder::new(
                            m,
                            config,
                            &device,
                            control_receiver,
                            packet_receiver,
                            packet_decoder,
                            ffmpeg_next::util::format::sample::Sample::F32(
                                ffmpeg_next::util::format::sample::Type::Packed
                            ),
                            output_channel_layout,
                            69,
                            3,
                            bb
                        ),
                        format @ _ => todo!("unsupported cpal output format {:#?}", format),
                    };

                    let packet_receiver_impl =
                        async {
                            ffmpeg_to_cpal_forwarder.stream(ctx.conn.get_raw_dpy(),drv,drb,bb,w,h).await;
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
                               // println!("SENDING DIE");
                                //con3.send(Player::CTL_DIE).await.unwrap_or(());
                               // println!("EXIT PLAY LOOP");
                                //return;
                            },
                            /*received_command = control_receiver.recv().fuse() => {
                                match received_command {
                                    Ok(Player::CTL_DIE) => {
                                        return;
                                    }
                                    _ => {}
                                }
                            }*/
                        }
                    }



                    //a2test("/mnt/slo/Videos/TV/Bad Audio/friends.s01e02.720p.bluray.x264-psychd-Obfuscated.mkv.mp4");
                });
                //con2.send_blocking(Player::CTL_DIE).unwrap_or(());
                sender2.send_blocking((Player::AUDIO_DEAD,"".to_string())).unwrap_or_default();
          //      println!("EXIT AUDIO REC LOOP");
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

impl Drop for ViAudioPlaybackThread {
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

trait ViFFMpegToCPalSampleForwarder {
    fn vi_forward(
        &mut self,
        audio_frame: ffmpeg_next::frame::Audio
    ) -> Pin<Box<dyn Future<Output = ()> + '_>>;
}

struct ViFFmpegToCPalForwarder {
    silent: bool,
    _cpal_stream: cpal::Stream,
    sample_producer: Producer<f32,Arc<SharedRb<f32,Vec<MaybeUninit<f32>>>>>,
//    ffmpeg_to_cpal_pipe: Box<dyn ViFFMpegToCPalSampleForwarder>,
    control_receiver: smol::channel::Receiver<i64>,
    packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
    packet_decoder: ffmpeg_next::decoder::Audio,
    resampler: ffmpeg_next::software::resampling::Context,
    pro_m: Rc<ProjectM>,
    frame_rate: u32,
    visual_frame_skip: u32,
    start_time: std::time::Instant,
    glpx: c_ulong,
    glctx: GLXContext,
    win: x::Window
}

impl ViFFmpegToCPalForwarder {
    fn vif(
        &mut self,
        //mut p: impl RbWrite<f32> + Rb<f32> + 'static,
        audio_frame: ffmpeg_next::frame::Audio
    ) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(async move {
            // Audio::plane() returns the wrong slice size, so correct it by hand. See also
            // for a fix https://github.com/zmwangx/rust-ffmpeg/pull/104.
            let mut expected_bytes =
                audio_frame.samples() * audio_frame.channels() as usize * core::mem::size_of::<f32>();
            if expected_bytes > audio_frame.data(0).len() { expected_bytes = audio_frame.data(0).len() };
            let sd: &[f32] = bytemuck::cast_slice(&audio_frame.data(0)[..expected_bytes]);
            let sdf: &[f32] = bytemuck::cast_slice(&audio_frame.data(0)[..ProjectM::pcm_get_max_samples() as usize]);
            
            while self.sample_producer.free_len() < sd.len() {
                smol::Timer::after(std::time::Duration::from_millis(16)).await;
            }

            // Buffer the samples for playback
            
            self.pro_m.pcm_add_float(sdf,2);
            self.sample_producer.push_slice(sd);

            // calculate frame time
            let frame_time = (ticks() - self.start_time).as_millis() as u32;
            // what do we need to hit target frame rate?
            let delay_needed = 1000 / self.frame_rate - frame_time;
            if delay_needed > 0  && delay_needed < 100 {
                delay(delay_needed);
            }
        })
    }

    fn new(
        m: Visual,
        config: cpal::SupportedStreamConfig,
        device: &cpal::Device,
        control_receiver: smol::channel::Receiver<i64>,
        packet_receiver: smol::channel::Receiver<ffmpeg_next::codec::packet::packet::Packet>,
        mut packet_decoder: ffmpeg_next::decoder::Audio,
        output_format: ffmpeg_next::util::format::sample::Sample,
        output_channel_layout: ffmpeg_next::util::channel_layout::ChannelLayout,
        frame_rate: u32,
        visual_frame_skip: u32,
        bb: x::Pixmap
    ) -> Self {
        let buffer = HeapRb::<f32>::new(1024*32);
        let (sample_producer, mut sample_consumer) = buffer.split();
        
        let cpal_stream = device
            .build_output_stream(
                &config.config(),
                move |data, _| {
                    let filled = sample_consumer.pop_slice(data);
                    data[filled..].fill(f32::EQUILIBRIUM);
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

        // and a preset playlist
        let ctx = &CTX;

        let fbc = get_glxfbconfig(
            ctx.conn.get_raw_dpy(),
            ctx.screen_n,
            &[
                /*GLX_X_RENDERABLE,1,
                GLX_X_VISUAL_TYPE,
                GLX_TRUE_COLOR,
                GLX_DRAWABLE_TYPE,GLX_PIXMAP_BIT|GLX_WINDOW_BIT,
                GLX_RENDER_TYPE, GLX_RGBA_BIT, GLX_RED_SIZE, 8, GLX_GREEN_SIZE, 8, GLX_BLUE_SIZE, 8,
                GLX_DOUBLEBUFFER, 1,
                0*/
                GLX_X_RENDERABLE,
                1,
                GLX_DRAWABLE_TYPE,
                GLX_PIXMAP_BIT,
                GLX_RENDER_TYPE|GLX_WINDOW_BIT,
                GLX_RGBA_BIT,
                GLX_X_VISUAL_TYPE,
                GLX_TRUE_COLOR,
                GLX_RED_SIZE,
                8,
                GLX_GREEN_SIZE,
                8,
                GLX_BLUE_SIZE,
                8,
                GLX_ALPHA_SIZE,
                8,
                GLX_DEPTH_SIZE,
                24,/*
                GLX_STENCIL_SIZE,
                8,
                GLX_DOUBLEBUFFER,
                1,*/
                0
            ],
        );

        unsafe {
            xlib::XSync(ctx.conn.get_raw_dpy(), xlib::False);
        }

        let glx_exts =
            unsafe { CStr::from_ptr(glXQueryExtensionsString(ctx.conn.get_raw_dpy(), ctx.screen_n)) }
                .to_str()
                .unwrap();

        if !check_glx_extension(&glx_exts, "GLX_ARB_create_context") {
            panic!("could not find GLX extension GLX_ARB_create_context");
        }

        // with glx, no need of a current context is needed to load symbols
        // otherwise we would need to create a temporary legacy GL context
        // for loading symbols (at least glXCreateContextAttribsARB)
        let glx_create_context_attribs: GlXCreateContextAttribsARBProc =
            unsafe { std::mem::transmute(load_gl_func("glXCreateContextAttribsARB")) };

        // loading all other symbols
        unsafe {
            gl::load_with(|n| load_gl_func(&n));
        }

        if !gl::GenVertexArrays::is_loaded() {
            panic!("no GL3 support available!");
        }

        // installing an event handler to check if error is generated
        unsafe {
            CTX_ERROR_OCCURED = false;
        }

        let old_handler = unsafe { xlib::XSetErrorHandler(Some(ctx_error_handler)) };

        let context_attribs: [c_int; 5] = [
            GLX_CONTEXT_MAJOR_VERSION_ARB as c_int,
            4,
            GLX_CONTEXT_MINOR_VERSION_ARB as c_int,
            0,
            0
        ];

        let gctx = unsafe {
            glx_create_context_attribs(
                ctx.conn.get_raw_dpy(),
                fbc,
                ptr::null_mut(),
                xlib::True,
                &context_attribs[0] as *const c_int,
            )
        };

        ctx.collect();

        unsafe {
            xlib::XSync(ctx.conn.get_raw_dpy(), xlib::False);
            xlib::XSetErrorHandler(std::mem::transmute(old_handler));
        }

        if gctx.is_null() || unsafe { CTX_ERROR_OCCURED } {
            panic!("error when creating gl-3.0 context");
        }

        if unsafe { glXIsDirect(ctx.conn.get_raw_dpy(), gctx) } == 0 {
            panic!("obtained indirect rendering context")
        }

        let mut glpx = 0;
        unsafe {
            let vi_ptr: *mut xlib::XVisualInfo = glXGetVisualFromFBConfig(ctx.conn.get_raw_dpy(), fbc);
            glpx = glXCreateGLXPixmap(ctx.conn.get_raw_dpy(),vi_ptr,bb.resource_id() as XID);
            println!("Current: {}",m.window.resource_id());
            glXMakeCurrent(ctx.conn.get_raw_dpy(),m.window.resource_id() as XID, gctx);
        }
        
        let pm = Rc::new(ProjectM::create());
        pm.set_fps(frame_rate/visual_frame_skip);
        pm.set_window_size(m.width as usize, m.height as usize);

        Self {
            silent: false,
            _cpal_stream: cpal_stream,
            sample_producer,
            //ffmpeg_to_cpal_pipe: Box::new(sample_producer),
            control_receiver,
            packet_receiver,
            packet_decoder,
            resampler,
            pro_m: pm,
            glpx,
            glctx: gctx,
            win: m.window,
            frame_rate,
            visual_frame_skip,
            start_time: std::time::Instant::now()
        }
    }

    async fn stream(&mut self,dpy:*mut _XDisplay, drw: x::Drawable, drb: x::Drawable, bb: x::Pixmap,w: u16,h: u16) {
        let ctx = &CTX;
        
        let rlim = 120*self.frame_rate;
        let mut rsc = rlim;
        let mut playlist = projectm::playlist::Playlist::create(&self.pro_m);
        playlist.add_path("../assets/pr-presets/classic", true);
        playlist.play_random();

        self.start_time = ticks();
        loop {
            // Receive the next packet from the packet receiver channel.
            let Ok(packet) = self.packet_receiver.recv().await else { break };
            // Send the packet to the decoder.
            self.packet_decoder.send_packet(&packet).unwrap();
            // Create an empty frame to hold the decoded audio data.
            let mut decoded_frame = ffmpeg_next::util::frame::Audio::empty();
            // Continue receiving decoded frames until there are no more available.
            let mut shot = false;

            while self.packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                rsc-=1;
                if rsc<0 {
                    playlist.play_random();
                    rsc = rlim;
                }
                
                if (rsc % self.visual_frame_skip) == 0
                {
                    unsafe {
                        //glXMakeCurrent(ctx.conn.get_raw_dpy(),self.win.resource_id() as XID, self.glctx);
                        self.pro_m.render_frame();
                        glXSwapBuffers(dpy, self.win.resource_id() as XID);

                        glXMakeCurrent(ctx.conn.get_raw_dpy(), self.glpx, self.glctx);
                        self.pro_m.render_frame();
                        glXMakeCurrent(ctx.conn.get_raw_dpy(), self.win.resource_id() as XID, self.glctx);
                    }
                }
                
                shot = true;
                futures::select! {
                    _ = futures::FutureExt::fuse(smol::Timer::after(std::time::Duration::from_millis(4))) => {}
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
                    self.vif(resampled_frame).await;
                    //self.ffmpeg_to_cpal_pipe.vi_forward(resampled_frame).await; 
                }
                self.start_time = ticks();
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