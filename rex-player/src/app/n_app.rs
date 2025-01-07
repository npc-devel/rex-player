use std::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver};
use std::thread::{Scope,Thread};

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
        thread::sleep(time::Duration::from_millis(1));
    }
    fn run(&mut self) {
        let mut width = 1280;
        let mut height = 720;

        let mut s_ply = Nscene::new("media-full.view");
        let mut v_medias= s_ply.build_in(&mut self.ctx,self.window);
        s_ply.anchor_fit_to(&mut self.ctx,0,0,width,height,&mut v_medias);

        let mut s_ovl = Nscene::new("osd.view");
        let mut v_controls= s_ovl.build_in(&mut self.ctx,self.window);
        s_ovl.anchor_fit_to(&mut self.ctx,0,0,width,height,&mut v_controls);
        let mut controls_by_window :HashMap<x::Window,u64> = nmap!();
        for v in v_controls.iter() {
            controls_by_window.insert(v.1.window,v.1.key);
        }
        let mut medias: idvec!() = s_ply.select("media");

        thread::scope(|s|{
            let v_m = v_medias.clone();
            let mut ctx = &self.ctx;
            let icons: idvec!() = s_ovl.select("i");
            let mut sen_t:Vec<Sender<String>> = vec![];

            for mi in medias {

                let (tx, rx) = mpsc::channel();
                sen_t.push(tx);
                s.spawn( move || {
                    let m = v_m.get(&mi).unwrap();
                   // loop {
                        Lffmpeg::stream_file(ctx, &rx, Drawable::Pixmap(m.buf), Drawable::Window(m.window), m.width as u32, m.height as u32, "/home/ppc/Videos/Samples/50MB_1080P_THETESTDATA.COM_mp4_new.mp4").expect("Bad file");
                        println!("Looping");
                   // }
                });
            }
            s.spawn(  || {
            let mut medias: idvec!() = s_ply.select("media");
                loop {
                    if !medias.is_empty() {
                        let bb = v_m.get(&medias[0]).unwrap().buf;
                        for i in &icons {
                            let vi = v_controls.get(&i).unwrap();
                            let cgc = Nreq::new_masked_gc(ctx, Drawable::Window(self.window), vi.inv_mask);
                            ctx.copy(cgc, Drawable::Pixmap(bb), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }
                    let ev = ctx.wait_event();
                    match ev.code {
                        Nevent::RESIZE => {
                            println!("RESIZE {:?}",ev.window);
                            if width!=ev.width || height!=ev.height {
                                width = ev.width;
                                height = ev.height;
                                //   let mut v_controls:vismap!() = nmap!();
                                s_ovl.anchor_fit_to(ctx, 0, 0, width, height, &mut v_controls);
                                s_ply.anchor_fit_to(ctx, 0, 0, width, height, &mut v_medias);
                                for r in sen_t.iter() {
                                    r.send(format!("size={width} {height}"));
                                }
                            }
                        }
                        Nevent::RENDER => {
                            let vio = controls_by_window.get(&ev.window);
                            if vio.is_some() {
                                let vi = v_controls.get(vio.unwrap()).unwrap();
                                //     let cgc = Nreq::new_masked_gc(ctx, Drawable::Window(self.window), vi.mask);
                                ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                            }
                        }
                        _ => {}
                    }
                    self.idle();
                }
            });
       });
    }
}