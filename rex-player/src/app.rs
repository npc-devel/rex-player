use std::thread::spawn;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::{Sender,Receiver};
use std::thread::{Scope,Thread};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::time::Duration;

use xcb::x::Visualid;
use xcb::{x, Connection, Event, Request, Xid};
use xcb::Event::X;
use xcb::x::{Cw, Drawable, Gc, Gcontext};
use xcb::x::Event::Expose;
use xcb::x::ImageFormat;
use xcb::x::Window;

use ffmpeg_next as ffmpeg;
use ffmpeg::Error::*;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next::codec::profile::JPEG2000::CStreamNoRestriction;
use ffmpeg_next::decoder::video;
use ffmpeg_next::{software, Error};
use ffmpeg_next::format::context::input;

use image::{DynamicImage, Rgba, ImageReader, EncodableLayout};
use image::GenericImageView;
use image::imageops::FilterType;

use rhai::{Engine, EvalAltResult};
use json::{array, JsonValue};
use rand::{random, thread_rng, Rng};

include!("rhai.rs");
include!("windowing/xcb.rs");
include!("visuals/scene.rs");
include!("ffmpeg.rs");

struct App {
    ctx: Xcb,
    pub window: x::Window,
    width:u16,
    height:u16,
    players: Layer,
    overlay: Layer,
    ffms: Vec<(x::Drawable,FfMpeg)>
}

impl App {
    fn new(w:u16,h:u16)-> Self {
        let mut ctx = Xcb::new();
        let window = ctx.new_window(0xFF000033);
        ctx.prepare(window);
      //  let (tx,rx) = mpsc::channel();
        let players = Layer::new("media-quad.view",&mut ctx,window,0,w,h);
        let overlay = Layer::new("osd.view",&mut ctx,window,0,w,h);

        Self {
            width:w,
            height:h,
            ctx,
            window,
            players,
            overlay,
            ffms: vec![]
        }
    }
    fn clean_up(&mut self) {

    }
    fn prepare(&mut self) {
        ffmpeg::init().expect("TODO: panic message");
        
        self.players.fit_all(&self.ctx,self.width,self.height);
        self.overlay.fit_all(&self.ctx,self.width,self.height);
        self.ctx.show(self.window);
    }
    fn idle(&self) {
        self.ctx.collect();
        //thread::sleep(time::Duration::from_millis(1));
    }

    fn run(&mut self) {
        let mut ctx = &self.ctx;
        let mut li = 0;
        loop {
            let ev = ctx.wait_event();
            li+=1;
            match ev.code {
                XcbEvent::NONE => {
                    for mut f in self.ffms.iter_mut() {
                      //  println!("!");
                        f.1.wait_events(ctx);
                        if f.1.dst != x::Drawable::none() {
                            ctx.copy(ctx.gc, f.1.dst, f.0, 0, 0, 0, 0, f.1.w as u16, f.1.h as u16);
                        }
                    }

                    self.idle();
                }
                XcbEvent::RESIZE => {
                    if self.width!=ev.width || self.height!=ev.height {
                        println!("RESIZE {}x{}",ev.width,ev.height);
                        self.width = ev.width;
                        self.height = ev.height;
                        self.players.fit_all(ctx,self.width,self.height);
                        self.overlay.fit_all(ctx,self.width,self.height);

                        let medias  = self.players.select("media");
                   //     println!("{:?}",medias.len());
                        if self.ffms.is_empty() {
                            for m in medias {
                                self.ffms.push((Drawable::Window(m.window),FfMpeg::new(ctx, &asset!("sample","mp4"), m.width as u32,m.height as u32)));
                            }
                        } else {
                            let mut idx = 0;
                            for m in medias {
                                let fo = self.ffms.get_mut(idx);
                                if fo.is_some() {
                                    fo.unwrap().1.rescale(m.width as u32, m.height as u32);
                                }
                                idx += 1;
                            }
                        }
                    /*    self.ffms.clear();

                        let medias  = self.players.select("media");
                        for m in medias {
                            self.ffms.push((Drawable::Window(m.window),FfMpeg::new(ctx, &asset!("loader","mp4"), m.width as u32,m.height as u32)));
                        }*/

                       // self.players.anchor_fit_to(ctx, 0, 0, self.width, self.height);
                     /*   let bb = &mut self.players.controls.get(&medias[0]).unwrap().buf.resource_id();
                        for r in senders.iter() {
                            r.send(format!("buf={bb} {} {}",self.width,self.height)).unwrap();
                        }*/
                    }
                }
                XcbEvent::RENDER => {
                    let vio = self.overlay.visual(ev.window.resource_id());
                    if vio.is_some() {
                        let vi = vio.unwrap();//self.overlay.controls.get(vio.unwrap()).unwrap();
                        if vi.buf != x::Pixmap::none() {
                            ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }
                }
                _ => {}
            }
        }
    }
   /* fn runo(&mut self) {
        thread::scope(|s|{
            let mut ctx = &self.ctx;
            let mut medias: idvec!() = self.players.select("media");
            let icons: idvec!() = self.overlay.select("i");
            let mut senders:Vec<Sender<String>> = vec![];
            let ctls = self.players.controls.clone();
            let mut medias: idvec!() = self.players.select("media");
            for mi in &medias {
                let mut s_drw = Drawable::Pixmap(self.players.controls.get(mi).unwrap().buf);
                let d_drw = Drawable::Window(self.players.controls.get(mi).unwrap().window);
                let mut mw:u32 = self.players.controls.get(mi).unwrap().width.clone() as u32;
                let mut mh:u32 = self.players.controls.get(mi).unwrap().height.clone() as u32;
               // senders.push(tx);
                s.spawn(move|| {
                    let (tx, rx) = mpsc::channel();

                    loop {
                        let msg = FfMpeg::stream_file(
                                                    ctx,
                                                    &rx,
                                                    s_drw,
                                                    d_drw,
                                                    mw,
                                                    mh,
                                                    &asset!("loader","mp4"));
                        if !msg.is_empty() {
                            println!("sizing {msg}");
                            let ma = msg.split('=').collect::<Vec<&str>>();
                            let va = ma[1].split(' ').collect::<Vec<&str>>();
                            match ma[0] {
                                "buf" => {
                                    println!("sizing {}",va[0]);
                                    let res = u32::from_str_radix(va[0], 10).unwrap();
                                 /*   let mo = self.players.resmap.get(&res);
                                    if mo.is_some() {
                                        s_drw = Drawable::Pixmap(*mo.unwrap());
                                        mw = u32::from_str_radix(va[1], 10).unwrap();
                                        mh = u32::from_str_radix(va[2], 10).unwrap();
                                    }*/
                                }
                                _ => {}
                            }
                        }
                    }
                  //  Lffmpeg::loop_file(&&resmap, ctx, &rx, s_drw, d_drw, mw as u32, mh as u32, &asset!("loader","mp4"));
                });
            }


            let bb = self.players.controls.get(&medias[0]).unwrap().buf;

          /*  s.spawn( move || {
                let ia = &icons[0..];
                loop {
                    for i in ia {
                        let vi = self.overlay.controls.get(&i).unwrap();
                        let cgc = Nreq::new_masked_gc(ctx, ctx.drawable, vi.inv_mask);
                        ctx.copy(cgc, Drawable::Pixmap(bb), Drawable::Window(vi.window), 0, 0, 0, 0, vi.width, vi.height);
                    }
                    thread::sleep(time::Duration::from_millis(16));
                }
            });*/

            loop {
                let ev = ctx.wait_event();
                match ev.code {
                    XcbEvent::NONE => {
                        self.idle();
                    }
                    XcbEvent::RESIZE => {
                        if self.width!=ev.width || self.height!=ev.height {
                            println!("RESIZE {:?}",ev.window);
                            self.width = ev.width;
                            self.height = ev.height;
                            self.overlay.anchor_fit_to(ctx, &self.rx,0, 0, self.width, self.height);
                            self.players.anchor_fit_to(ctx, &self.rx,0, 0, self.width, self.height);
                            let bb = &mut self.players.controls.get(&medias[0]).unwrap().buf.resource_id();
                            for r in senders.iter() {
                                r.send(format!("buf={bb} {} {}",self.width,self.height)).unwrap();
                            }
                        }
                    }
                    XcbEvent::RENDER => {
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
    }*/
}