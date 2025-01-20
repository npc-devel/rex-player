use ffmpeg_next::format::context::Input;

trait SampleFormatConversion {
    fn as_ffmpeg_sample(&self) -> FFmpegSample;
}

impl SampleFormatConversion for SampleFormat {
    fn as_ffmpeg_sample(&self) -> FFmpegSample {
        match self {
            Self::I16 => FFmpegSample::I16(SampleType::Packed),
            Self::U16 => {
                panic!("ffmpeg resampler doesn't support u16")
            },
            Self::F32 => FFmpegSample::F32(SampleType::Packed),
            &_ => {
                FFmpegSample::I16(SampleType::Packed)
            }
        }
    }
}


struct FfMpeg {
    video_scalar: software::scaling::Context,
    video_decoder: video::Video,
    //audio_decoder: audio::Audio,
    //audio_resample: software::resampling::Context,
    input_ctx: input::Input,
    frame_index: u32,
    video_stream_index: usize,
    //audio_stream_index: usize,
    dst: x::Drawable,
    w: u32,
    h: u32,
    rw: u32,
    rh: u32
}
impl FfMpeg {
    fn receive_and_process_decoded_frames(&mut self, ctx:&Xcb) {
        let mut decoded = Video::empty();
        if self.video_decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = Video::empty();
            self.video_scalar.run(&decoded, &mut rgb_frame).unwrap();
            let data = rgb_frame.data(0);
            let pl = data.len() as u32/4;
            self.rh = rgb_frame.plane_height(0);
            self.rw = pl/self.rh;
            if self.dst == x::Drawable::none() {
                self.dst = Drawable::Pixmap(ctx.new_pixmap(self.rw as u16,self.rh as u16));
            }
            ctx.fill(ctx.gc, self.dst, data, 0, 0, self.rw as u16, self.rh as u16);
            self.frame_index += 1;
        }
    }

    fn init() {
        ffmpeg::init().unwrap();
    }

    fn rescale(&mut self, w:u32, h:u32) {
        self.video_scalar = Context::get(
            self.video_decoder.format(),
            self.video_decoder.width(),
            self.video_decoder.height(),
            Pixel::BGRA,
            w,
            h,
            Flags::LANCZOS
        ).unwrap();
        self.w = w;
        self.h = h;
        self.dst = Drawable::none();
    }

    fn init_cpal() -> (cpal::Device, cpal::SupportedStreamConfig) {
        let device = cpal::default_host()
            .default_output_device()
            .expect("no output device available");

        let supported_config_range = device.supported_output_configs()
            .expect("error querying audio output configs")
            .next()
            .expect("no supported audio config found");

        (device, supported_config_range.with_max_sample_rate())
    }
    
    
    fn open(file:&str)->Result<Input,Error> {
        println!("checking {file}");
        let input_ctx = input(file)?;
        let a_input = input_ctx
            .streams()
            .best(Type::Audio)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let input = input_ctx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        Ok(input_ctx)
    }
    
    fn new(ctx:&Xcb,input_ctx:Input,w:u32,h:u32)->Self {
        let mut frame_index = 0;
        let a_input = input_ctx
            .streams()
            .best(Type::Audio)
            .ok_or(ffmpeg::Error::StreamNotFound).unwrap();

        let input = input_ctx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound).unwrap();
        let video_stream_index = input.index();
//        let audio_stream_index = a_input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap().decoder();
        let video_decoder = context_decoder.video().unwrap();

  //      let a_context_decoder = ffmpeg::codec::context::Context::from_parameters(a_input.parameters()).unwrap().decoder();
    //    let audio_decoder = a_context_decoder.audio().unwrap();

        let mut video_scalar = Context::get(
            video_decoder.format(),
            video_decoder.width(),
            video_decoder.height(),
            Pixel::BGRA,
            w,
            h,
            Flags::BILINEAR
        ).unwrap();

      /*  let (device, stream_config) = Self::init_cpal();
        let mut audio_resample = ResamplingContext::get(
            audio_decoder.format(),
            audio_decoder.channel_layout(),
            audio_decoder.rate(),

            stream_config.sample_format().as_ffmpeg_sample(),
            audio_decoder.channel_layout(),
            stream_config.sample_rate().0
        ).unwrap();*/

        Self {
            video_scalar,
            video_decoder,
            //audio_decoder,
            //audio_resample,
            input_ctx,
            frame_index,
            video_stream_index,
            //audio_stream_index,
            dst:x::Drawable::none(),
            w,
            h,
            rw: 0,
            rh: 0
        }
    }

    fn wait_events(&mut self,ctx: &Xcb)->bool {
        for (stream, packet) in self.input_ctx.packets() {
            if stream.index() == self.video_stream_index {
                self.video_decoder.send_packet(&packet);
                self.receive_and_process_decoded_frames(ctx);
                return true;
            }
            //else if stream.index() == self.audio_stream_index {
             //   self.audio_decoder.send_packet(&packet).unwrap();
              //  return true;
           // }
        }
        //thread::sleep(Duration::from_millis(10));
        false
    }

    fn stop(&mut self) {
        self.video_decoder.send_eof().unwrap();
        //
        // self.audio_decoder.send_eof().unwrap();
    }

      /* fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("frame{}.ppm", index))?;
        file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
        file.write_all(frame.data(0))?;
        Ok(())
    }*/
}