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
    pub window: x::Window,
    width:u16,
    height:u16,
    players: Nscene,
    overlay: Nscene
}

impl Napp {
    fn new(w:u16,h:u16)-> Self {
        let mut ctx = Nxcb::new();
        ctx.prepare();
        let window = Nreq::new_window(&mut ctx,0xFF000033);
        let players= Nscene::build("media-full.view",&mut ctx,window,w,h);
        let overlay= Nscene::build("osd.view",&mut ctx,window,w,h);
        Self {
            width:w,
            height:h,
            ctx,
            window,
            players,
            overlay
        }
    }
    fn clean_up(&mut self) {

    }
    fn prepare(&mut self) {
        self.ctx.show(self.window);
    }
    fn idle(&self) {
        self.ctx.collect();
        thread::sleep(time::Duration::from_millis(1));
    }
    fn run(&mut self) {
        thread::scope(|s|{
            let mut ctx = &self.ctx;

            let mut medias: idvec!() = self.players.select("media");
            let icons: idvec!() = self.overlay.select("i");
            let mut senders:Vec<Sender<String>> = vec![];

            let c_controls = self.overlay.controls.clone();


            for mi in &medias {
                let (tx, rx) = mpsc::channel();
                senders.push(tx);
                let m = self.players.controls.get(&mi).unwrap();
                let s_drw = Drawable::Pixmap(m.buf);
                let d_drw = Drawable::Window(m.window);
                let mw = m.width.clone();
                let mh = m.height.clone();
                let resmap = self.players.resmap.clone();

                s.spawn(move|| {
                    Lffmpeg::loop_file(&resmap, ctx, &rx, s_drw, d_drw, mw as u32, mh as u32, &asset!("sample","mp4"));
                });
            }

            {
                let c_controls = self.overlay.controls.clone();
                let icons = icons.clone();
                let bb = self.players.controls.get(&medias[0]).unwrap().buf;
                s.spawn(move || {
                    loop {
                        for i in &icons {
                            let vi = c_controls.get(&i).unwrap();
                            let cgc = Nreq::new_masked_gc(ctx, ctx.drawable, vi.inv_mask);
                            ctx.copy(cgc, Drawable::Pixmap(bb), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                        }
                        thread::sleep(time::Duration::from_millis(16));
                    }
                });
            }

            loop {
                let ev = ctx.wait_event();
                match ev.code {
                    Nevent::NONE => {
                        self.idle();
                    }
                    Nevent::RESIZE => {
                        if self.width!=ev.width || self.height!=ev.height {
                            println!("RESIZE {:?}",ev.window);
                            self.width = ev.width;
                            self.height = ev.height;
                            self.overlay.anchor_fit_to(ctx, 0, 0, self.width, self.height);
                            self.players.anchor_fit_to(ctx, 0, 0, self.width, self.height);
                            let bb = &mut self.players.controls.get(&medias[0]).unwrap().buf.resource_id();
                            for r in senders.iter() {
                                r.send(format!("buf={bb} {} {}",self.width,self.height));
                            }
                        }
                    }
                    Nevent::RENDER => {
                        let vio = self.overlay.window_ids.get(&ev.window);
                        if vio.is_some() {
                            let vi = self.overlay.controls.get(vio.unwrap()).unwrap();
                            ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}