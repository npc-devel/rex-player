use std::*;
use std::collections::HashMap;
use std::thread::Scope;

include!("../script/l_rhai.rs");
include!("../windowing/n_xcb.rs");
include!("../media/l_ffmpeg.rs");

struct Napp {
    ctx: Nxcb,
    pub window: x::Window
}

impl Napp {
    fn new()-> Self {
        Self {
            ctx: Nxcb::new(),
            window: x::Window::none()
        }
    }
    fn clean_up(&mut self) {

    }
    fn prepare(&mut self) {
        self.ctx.prepare();
        self.window = Nreq::new_window(&mut self.ctx,0xFF000033);
        self.ctx.show(self.window);
    }
    fn idle(&self) {
        self.ctx.collect();
        thread::sleep(time::Duration::from_millis(3));
    }
    fn run(&mut self) {
        let ctx = &mut self.ctx;
        let mut sce = Nscene::new("osd.view");
        let mut visuals= sce.build_in(ctx,self.window);
        sce.anchor_fit_to(ctx,0,0,1280,720,&mut visuals);
        let mut medias: idvec!() = sce.select("media");

        thread::scope(|s|{
            let tctx = &mut *ctx;
            for mi in medias {
                let m = visuals.get(&mi).unwrap();
                s.spawn(move||{
                    Lffmpeg::stream_file(tctx, Drawable::Window(m.window), "/home/ppc/Videos/Samples/50MB_1080P_THETESTDATA.COM_mp4_new.mp4").expect("Bad file");
                });
            }

        });
      //  let dlg = Nreq::new_sub_window(&self.win_ctx,self.window,0xFF001100);
      //  ctx.show(self.window);
        //let vid = l_ffmpeg::new(Drawable::Window(dlg));
       // let spr = Nsprite::new(&self.ctx,"jumbo");
      //  loop {
            //spr.dump(&self.win_ctx,self.window);
     //       self.idle();
          //  let vid = l_ffmpeg::new();
         //   vid.stream_file(&asset!("sample","mp4"),self).expect("Bad file");
      //  }
    }
}