use ffmpeg_next as ffmpeg;

use ffmpeg::Error::*;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::time::Duration;
use ffmpeg_next::codec::profile::JPEG2000::CStreamNoRestriction;
use ffmpeg_next::Error;

struct Lffmpeg {
}
impl Lffmpeg {
    fn new()->Self {
        Self  {
        }
    }
    fn render_frame(&self, rgb_frame:&Video, frame_index:u64, app:&Napp) {

    }
    fn stream_file(ctx:&Nxcb, rx:&Receiver<String>, buf:Drawable, win:Drawable, mut des_w:u32, mut des_h:u32, file:&str) -> Result<(),ffmpeg::Error> {

        let mut frame_index = 0;
        println!("Start {des_w} {des_h}");
  //      loop {
            ffmpeg::init().unwrap();
            println!("Tryplay {des_w} {des_h}");
            if let Ok(mut ictx) = input(file) {
                let input = ictx
                    .streams()
                    .best(Type::Video)
                    .ok_or(ffmpeg::Error::StreamNotFound)?;
                let video_stream_index = input.index();
                let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
                let mut decoder = context_decoder.decoder().video()?;

            let dst_w = des_w.clone();
            let dst_h = des_h.clone();







                    let mut scaler = Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        Pixel::BGRA,
                        dst_w,
                        dst_h,
                        Flags::BILINEAR,
                    )?;


                    let mut receive_and_process_decoded_frames =
                        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                            let mut reinit = false;
                            let mut decoded = Video::empty();

                            while decoder.receive_frame(&mut decoded).is_ok() {
                                // Nreq::hide(ctx,ctl);
                                let mut rgb_frame = Video::empty();
                                scaler.run(&decoded, &mut rgb_frame)?;

                                ctx.fill(ctx.gc, buf, &rgb_frame.data(0), 0, 0, dst_w as u16, dst_h as u16);
                                ctx.copy(ctx.gc, buf, win, 0, 0, 0, 0, dst_w as u16, dst_h as u16);

                                let tr = rx.try_recv();
                                if tr.is_ok() {
                                    let m = rx.recv().unwrap();
                                    println!("msg {m}");

                                    let ma = m.split('=').collect::<Vec<&str>>();
                                    let va = ma[1].split(' ').collect::<Vec<&str>>();
                                    match ma[0] {
                                        "size" => {
                                            des_w = u32::from_str_radix(va[0],10).unwrap();
                                            des_h = u32::from_str_radix(va[1],10).unwrap();
                                            reinit = true;
                                            break;
                                        }
                                        _ => {}
                                    }
                                }

                                thread::sleep(Duration::from_millis(1));

                                frame_index += 1;
                            }
                                if reinit { frame_index = 0; return Err(Error::Bug) }
//else {
                                Ok(())
                            //}
                        };

                    for (stream, packet) in ictx.packets() {
                        if stream.index() == video_stream_index {
                            decoder.send_packet(&packet)?;
                            if receive_and_process_decoded_frames(&mut decoder).is_err() { break }
                        }
                    }
                    decoder.send_eof()?;
                    receive_and_process_decoded_frames(&mut decoder)?;
                    frame_index = 0;
                }
     //       }

        Ok(())
    }

   /* fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("frame{}.ppm", index))?;
        file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
        file.write_all(frame.data(0))?;
        Ok(())
    }*/
}