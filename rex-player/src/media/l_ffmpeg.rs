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

struct Lffmpeg {
 //   drawable: Drawable
}
impl Lffmpeg {
    fn new()->Self {
        Self  {
   //         drawable
        }
    }
    fn render_frame(&self, rgb_frame:&Video, frame_index:u64, app:&Napp) {

    }
    fn stream_file(ctx:&Nxcb,buf:Drawable,win:Drawable,dst_w:u32,dst_h:u32,file:&str) -> Result<(),ffmpeg::Error> {
        ffmpeg::init().unwrap();
     //   let spr = Nsprite::new(&app.ctx,"jumbo");
        if let Ok(mut ictx) = input(file) {
            let input = ictx
                .streams()
                .best(Type::Video)
                .ok_or(ffmpeg::Error::StreamNotFound)?;
            let video_stream_index = input.index();

            let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
            let mut decoder = context_decoder.decoder().video()?;

            let mut scaler = Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::BGRA,
                dst_w,
                dst_h,
                Flags::BILINEAR,
            )?;

          //  let ctx = &app.ctx;

            let mut frame_index = 0;
          /*  let fbo = Nreq::new_pixmap(ctx,1280,720);




            let fbo_d = Drawable::Pixmap(fbo);

            let msk = Nreq::new_mask(ctx,"circle");
            let cgc = Nreq::new_masked_gc(ctx,Drawable::Window(app.window),msk);
            let img = Nreq::new_img_backgrounded(ctx,"jumbo",0xFF0088AA);
            let ctl = Nreq::new_sub_window(ctx,app.window,0xFF000000);
           // let ovl = Nreq::new_sheer_window(ctx,app.window,0xFF808080);

            let img_d = Drawable::Pixmap(img);
            let ctl_d = Drawable::Window(ctl);
            let app_d = Drawable::Window(app.window);
            ctx.show(ctl);*/
        //    ctx.show(ovl);
            let mut receive_and_process_decoded_frames =
                |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                    let mut decoded = Video::empty();
               //     let drawable = Drawable::Window(app.window);
                 //   let gc = Nreq::new_gc(ctx,drawable);

                    while decoder.receive_frame(&mut decoded).is_ok() {
                       // Nreq::hide(ctx,ctl);
                        let mut rgb_frame = Video::empty();
                        scaler.run(&decoded, &mut rgb_frame)?;

                        ctx.fill(ctx.gc,buf, &rgb_frame.data(0),0,0,dst_w as u16,dst_h as u16);
                        ctx.copy(ctx.gc,buf, win,0,0,0,0,dst_w as u16,dst_h as u16);
                     //   ctx.copy(ctx.gc,fbo_d, ctl_d,0,0,0,0,96,96);
                    //    ctx.copy(cgc,img_d, ctl_d,0,0,0,0,96,96);
                      /*  ctx.request(&x::PutImage {
                            format: x::ImageFormat::ZPixmap,
                            drawable: fbo_d,
                            gc: ctx.gfx_ctx,
                            width: 1280,
                            height: 720,
                            dst_x: 0,
                            dst_y: 0,
                            left_pad: 0,
                            depth: ctx.depth,
                            data: &rgb_frame.data(0).as_ref()
                        });*/
               /*       ctx.request(&x::PutImage {
                            format: x::ImageFormat::ZPixmap,
                            drawable: Drawable::Window(ctl),
                            gc: ctx.gfx_ctx,
                            width: 96,
                            height: 96,
                            dst_x: 0,
                            dst_y: 0,
                            left_pad: 0,
                            depth: ctx.depth,
                            data: &rgb_frame.data(0).as_ref()
                        });*/
                /*        ctx.request(&x::CopyArea {
                            src_drawable: fbo_d,
                            dst_drawable: Drawable::Window(app.window),
                            gc: ctx.gfx_ctx,
                            src_x: 0,
                            src_y: 0,
                            dst_x: 0,
                            dst_y: 0,
                            width: 1280,
                            height: 720,
                        });
                        ctx.request(&x::CopyArea {
                            src_drawable: fbo_d,
                            dst_drawable: Drawable::Window(ctl),
                            gc: ctx.gfx_ctx,
                            src_x: 0,
                            src_y: 0,
                            dst_x: 0,
                            dst_y: 0,
                            width: 96,F
                            height: 96,
                        });

                        ctx.request(&x::CopyArea {
                            src_drawable: Drawable::Pixmap(img),
                            dst_drawable: Drawable::Window(ctl),
                            gc: cgc,
                            src_x: 0,
                            src_y: 0,
                            dst_x: 0,
                            dst_y: 0,
                            width: 96,
                            height: 96,
                        });*/
//Nreq::show(ctx,ctl);
//spr.dump(&app.win_ctx,ctl);
                        //app.idle();
                        thread::sleep(Duration::from_millis(10));

                        frame_index += 1;
                    }
                    Ok(())
                };

            for (stream, packet) in ictx.packets() {
                if stream.index() == video_stream_index {
                    decoder.send_packet(&packet)?;
                    receive_and_process_decoded_frames(&mut decoder)?;
                }
            }
            decoder.send_eof()?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }

        Ok(())
    }

    fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("frame{}.ppm", index))?;
        file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
        file.write_all(frame.data(0))?;
        Ok(())
    }

}