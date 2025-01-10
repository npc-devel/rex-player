
struct FfMpeg {
    scaler: software::scaling::Context,
    decoder: video::Video,
    ictx: input::Input,
    frame_index: u32,
    video_stream_index: usize,
    dst: x::Drawable,
    w: u32,
    h: u32,
    rw: u32,
    rh: u32
}
impl FfMpeg {
    fn receive_and_process_decoded_frames(&mut self, ctx:&Xcb) {
        let mut decoded = Video::empty();
        if self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = Video::empty();
            self.scaler.run(&decoded, &mut rgb_frame);
            let data = rgb_frame.data(0);
            let plen = data.len() as u32/4;
            self.rh = rgb_frame.plane_height(0);
            self.rw = plen/self.rh;
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
        self.scaler = Context::get(
            self.decoder.format(),
            self.decoder.width(),
            self.decoder.height(),
            Pixel::BGRA,
            w,
            h,
            Flags::LANCZOS
        ).unwrap();
        self.w = w;
        self.h = h;
        self.dst = Drawable::none();
    }

    fn new(ctx:&Xcb, file:&str, w:u32, h:u32)->Self {
        let mut frame_index = 0;
        let mut ictx = input(file).unwrap();

        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound).unwrap();
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
        let decoder = context_decoder.decoder().video().unwrap();

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::BGRA,
            w,
            h,
            Flags::BILINEAR
        ).unwrap();

        Self {
            scaler,
            decoder,
            ictx,
            frame_index,
            video_stream_index,
            dst:x::Drawable::none(),
            w,
            h,
            rw: 0,
            rh: 0
        }
    }

    fn wait_events(&mut self,ctx: &Xcb) {
        for (stream, packet) in self.ictx.packets() {
            if stream.index() == self.video_stream_index {
                self.decoder.send_packet(&packet).unwrap();
                self.receive_and_process_decoded_frames(ctx);
                return;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    fn stop(&mut self) {
        self.decoder.send_eof().unwrap();
    }

      /* fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("frame{}.ppm", index))?;
        file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
        file.write_all(frame.data(0))?;
        Ok(())
    }*/
}