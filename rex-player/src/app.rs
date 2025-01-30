
use std::thread::spawn;
use std::collections::HashMap;
use std::sync::{mpsc, LazyLock};
use std::thread;
use std::sync::mpsc::{Sender,Receiver};
use std::thread::{Scope,Thread};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::time::Duration;

use xcb::x::Visualid;
use xcb::{x, Connection, Event, Request, Xid};
use xcb::Event::X;
use xcb::x::{Cw, Drawable, Gc, Gcontext};
use xcb::x::Event::Expose;
use xcb::x::ImageFormat;
use xcb::x::Window;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};

use ffmpeg_next as ffmpeg;
use ffmpeg::Error::*;
use ffmpeg::format::sample::Type as SampleType;
use ffmpeg_next::decoder::audio;
use ffmpeg::format::{Sample as FFmpegSample,input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::software::resampling::{context::Context as ResamplingContext};
use ffmpeg::util::frame::video::Video;
use ffmpeg::util::frame::audio::Audio;
use ffmpeg_next::codec::profile::JPEG2000::CStreamNoRestriction;
use ffmpeg_next::decoder::video;
use ffmpeg_next::{software, Error};
use ffmpeg_next::format::context::input;

use image::{DynamicImage, Rgba, ImageReader, EncodableLayout};
use image::GenericImageView;
use image::imageops::FilterType;

use rhai::{Engine, EvalAltResult,CustomType, TypeBuilder, INT};
use rhai::packages::Package;
use rhai_fs::FilesystemPackage;
use rhai_rand::RandomPackage;

use json::{array, JsonValue};
use rand::{random, thread_rng, Rng};

include!("rhai.rs");
include!("windowing/xcb.rs");
include!("visuals/style.rs");
include!("visuals/layer.rs");
include!("visuals/sprite.rs");
include!("ffmpeg.rs");

struct App {
    ctx: Xcb,
    pub window: x::Window,
    back_buffer: x::Pixmap,
    width:u16,
    height:u16,
    ffms: Vec<(x::Drawable,FfMpeg)>,
    //engine: Rhai,
    style: Style
}

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref all_layers: Mutex<HashMap<String,Layer>> = Mutex::new(HashMap::new());
}

#[macro_export]
macro_rules! layer_ref {
    () => { all_layers.lock().unwrap(); }
}
#[macro_export]
macro_rules! layer {
    ($x:expr,$y:expr) => {
        { $x.get($y).unwrap() }
    }
}
impl App {
    fn new(w:u16,h:u16)-> Self {
        let mut ctx = Xcb::new();
        let back_buffer = x::Pixmap::none();
        let window = ctx.new_window(0xFF101010);
        ctx.prepare(window);
        
        let style = Style::new(&ctx,"common");
        //let engine = Rhai::new();
        
        Self {
            width:w,
            height:h,
            ctx,
            window,
            back_buffer,
            ffms: vec![],
            //engine,
            style
        }
    }
    fn clean_up(&mut self) {

    }
    fn prepare(&mut self) {
        FfMpeg::init();

     //   for l in LAYERS.lock().unwrap().iter_mut() {
       //     l.1.fit_all(&self.ctx, &self.style, self.width, self.height);
            //l.1.root_visual.show(&self.ctx);
        //}
        //self.engine.register_app(self);
        self.ctx.show(self.window);
    }
    fn idle(&self) {
        self.ctx.collect();
        thread::sleep(Duration::from_millis(1));
    }

    fn run(&mut self,engine:&mut Rhai) {
        let mut ctx = &self.ctx;
        let mut li = 0;

        all_layers.lock().unwrap().insert("players".to_string(),Layer::new("media-quad.view", &mut ctx, self.window,0,0,self.width,self.height));
        all_layers.lock().unwrap().insert("overlay".to_string(),Layer::new("osd.view", &mut ctx, self.window,0,0,self.width,self.height));
        loop {
            all_layers.lock().unwrap()["overlay"].root_visual.show(ctx);

            let ev = ctx.wait_event();
            li+=1;
            match ev.code {
                XcbEvent::NONE => {
                        let mut idx = 0;
                        let bbd = Drawable::Pixmap(self.back_buffer);
                        let l = &all_layers.lock().unwrap()["players"];
                        for mut f in self.ffms.iter_mut() {
                            idx += 1;
                            if f.1.wait_events(ctx) {
                                let m = l.select("media")[idx];
                                if f.1.dst != x::Drawable::none() {
                                    ctx.copy(ctx.gc, f.1.dst, bbd, 0, 0, m.x, m.y, m.width, m.height);
                                }
                            } else {
                                let m = l.select("media")[idx];
                                loop {
                                    let file = &engine.exec(&m.content);
                                    let inp = FfMpeg::open(file);
                                    if inp.is_ok() {
                                        f.1 = FfMpeg::new(inp.unwrap(), m.width as u32, m.height as u32);
                                        break;
                                    }
                                }
                            }
                        }
                        let bbw = Drawable::Window(self.window);
                        ctx.copy(ctx.gc, bbd, bbw, 0, 0, 0, 0, self.width, self.height);
                        let l = &all_layers.lock().unwrap()["overlay"];
                        let mut icons = l.select("i");
                        icons.extend(l.select("lbl"));
                        for vi in icons {
                            let wd = Drawable::Window(vi.window);

                            if vi.inv_mask != x::Pixmap::none() {
                                let vd = Drawable::Pixmap(vi.buf);
                                let gc = ctx.new_gc(vd, vi.bg, vi.fg);
                                let mgc = ctx.new_masked_gc(wd, vi.mask, vi.fg, vi.bg);
                                let mgc_i = ctx.new_masked_gc(wd, vi.inv_mask, vi.fg, vi.bg);

                                ctx.rect(gc, wd, 0, 0, vi.width, vi.height);
                                ctx.copy(mgc_i, bbd, wd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                                ctx.copy(mgc, vd, wd, 0, 0, 0, 0, vi.width, vi.height);
                            } else if vi.buf != x::Pixmap::none() {
                                ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), wd, 0, 0, 0, 0, vi.width, vi.height);
                            }
                        }

                    self.idle();
                }
                XcbEvent::RESIZE => {
                    if self.width!=ev.width || self.height!=ev.height {
                   //     println!("RESIZE {}x{}",ev.width,ev.height);
                        self.width = ev.width;
                        self.height = ev.height;

                        self.back_buffer = ctx.new_pixmap(self.width,self.height);
                        ctx.map_bg(self.window,self.back_buffer);

                        //ctx.map_bg(self.window,s);
                        for l in all_layers.lock().unwrap().iter_mut() {
                            l.1.fit_all(ctx,&self.style,self.width,self.height);
                        }
                        //all_layers.get_mut("overlay").unwrap().fit_all(ctx,&self.style,self.width,self.height);
                        //players.fit_all(ctx,&self.style,self.width,self.height);
                        let l = &all_layers.lock().unwrap()["players"];
                        let medias = l.select("media");
                   //     println!("{:?}",medias.len());
                        if self.ffms.is_empty() {
                            for m in medias {
                                loop {
                                    let file = &engine.exec(&m.content);
                                    let inp = FfMpeg::open(file);
                                    if inp.is_ok() {
                                        self.ffms.push((Drawable::Window(m.window), FfMpeg::new(inp.unwrap(), m.width as u32, m.height as u32)));
                                        break;
                                    }
                                }
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