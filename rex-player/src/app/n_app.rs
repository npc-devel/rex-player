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
        let mut sce = Nscene::new("osd.view");
        let mut visuals= sce.build_in(&mut self.ctx,self.window);

        sce.anchor_fit_to(&mut self.ctx,0,0,1280,720,&mut visuals);

        let mut medias: idvec!() = sce.select("media");
       thread::scope(|s|{
            let mut ctx = &self.ctx;
            for mi in &medias {
                let m = visuals.get(&mi).unwrap();
                s.spawn(||{
                    Lffmpeg::stream_file(ctx, Drawable::Pixmap(m.buf),Drawable::Window(m.window), m.width as u32, m.height as u32, "/home/ppc/Videos/Samples/50MB_1080P_THETESTDATA.COM_mp4_new.mp4").expect("Bad file");
                });
            }

        let mut visuals_by_window :HashMap<x::Window,&Nvisual> = nmap!();
        for v in visuals.iter() {
            visuals_by_window.insert(v.1.window,&v.1);
        }


        let mut icons: idvec!() = sce.select("i");

            loop {
                if !medias.is_empty() {
                    for i in &icons {
                        let vi = visuals.get(&i).unwrap();
                        let cgc = Nreq::new_masked_gc(ctx, Drawable::Window(self.window), vi.mask);
                        ctx.copy(cgc, Drawable::Pixmap(visuals[&medias[0]].buf), Drawable::Window(vi.window), 0, 0, 0, 0, 64, 64);
                    }
                }
                let ev = ctx.wait_event();
                match ev.code {
                    Nevent::RENDER => {
                        let vio = visuals_by_window.get(&ev.window);
                        if vio.is_some() {
                            let vi = vio.unwrap();
                            ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), Drawable::Window(vi.window), 0, 0, 8, 0, vi.width, vi.height);
                        }
                        }
                    _ => {}
                }
                self.idle();
            }
       });
      //  let dlg = Nreq::new_sub_window(&self.win_ctx,self.window,0xFF001100);
      //  ctx.show(self.window);
        //let vid = l_ffmpeg::new(Drawable::Window(dlg));
       // let spr = Nsprite::new(&self.ctx,"jumbo");
      //
    }
}