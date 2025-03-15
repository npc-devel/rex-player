#![allow(warnings)]

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
use std::borrow::BorrowMut;
use std::sync::Arc;
use std::time::Instant;

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


use ffmpeg::{codec, decoder, frame, format, media};
use ffmpeg::software::resampling::context::Context as SwrContext;
use ffmpeg::format::sample::Type as SampleType;
use ffmpeg_next::decoder::audio;
use ffmpeg::format::{Sample as FFmpegSample,input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as VideoContext, flag::Flags};
use ffmpeg::software::resampling::{context::Context as AudioContext};
use ffmpeg::util::frame::video::Video;
use ffmpeg::util::frame::audio::Audio;
use ffmpeg_next::codec::profile::JPEG2000::CStreamNoRestriction;
use ffmpeg_next::decoder::video;
use ffmpeg_next::{software, Error};
use ffmpeg_next::format::context::input;
use ffmpeg_next::format::context::Input;
use futures::AsyncSeekExt;
use image::{DynamicImage, Rgba, ImageReader, EncodableLayout};
use image::GenericImageView;
use image::imageops::FilterType;

use rhai::{Engine, EvalAltResult, Scope as RScope, CustomType, TypeBuilder, INT};
use rhai::packages::Package;
use rhai_fs::FilesystemPackage;
use rhai_rand::RandomPackage;

use json::{array, JsonValue};
use rand::{random, thread_rng, Rng};
use xcb_util_cursor::{Cursor, CursorContext};

use xcb::ffi::xcb_generic_event_t;
use xcb::{self, dri2, glx};
use xcb::{Raw};

use x11::glx::*;
use x11::xlib;

use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::{ptr};
use std::rc::Rc;
use projectm::core::ProjectM;

use std::path::PathBuf;
use ffmpeg_next::{rescale, Rescale};
use futures::TryFutureExt;
use smol::stream::StreamExt;

use std::pin::Pin;

use bytemuck::Pod;
use cpal::{SampleRate, SizedSample};

use futures::future::OptionFuture;
use futures::FutureExt;


use ringbuf::{*,ring_buffer::*};
use std::mem::MaybeUninit;
use std::ops::DerefMut;
use x11::xlib::_XDisplay;
use std::future::Future;
use ffmpeg_next::{threading, ChannelLayout, Codec, Rational};
use ffmpeg_next::codec::debug;
use ffmpeg_next::format::sample::Type::Planar;
use ffmpeg_next::util::configuration;

const GLX_CONTEXT_MAJOR_VERSION_ARB: u32 = 0x2091;
const GLX_CONTEXT_MINOR_VERSION_ARB: u32 = 0x2092;

type GlXCreateContextAttribsARBProc = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    fbc: GLXFBConfig,
    share_context: GLXContext,
    direct: xlib::Bool,
    attribs: *const c_int,
) -> GLXContext;

fn ticks()->std::time::Instant {
    std::time::Instant::now()
}
fn delay(ms:u32) {
    thread::sleep(std::time::Duration::from_millis(ms as u64));
}

#[allow(dead_code)]
pub fn generate_random_audio_data(pm: &Rc<ProjectM>) {
    // Create a Vec<i16> with 1024 elements
    // two channels of 512 samples each
    let ms = ProjectM::pcm_get_max_samples() as usize / 2;
    let mut pcm_data: Vec<i16> = vec![0; ms as usize * 2];

    for i in 0..ms {
        if i % 2 == 1 {
            pcm_data[i * 2] = -(pcm_data[i * 2] as i32) as i16;
            pcm_data[i * 2 + 1] = -(pcm_data[i * 2 + 1] as i32) as i16;
        }
    }

    pm.pcm_add_int16(&pcm_data, 2);
}

unsafe fn load_gl_func(name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    let ptr: *mut c_void = std::mem::transmute(glXGetProcAddress(cname.as_ptr() as *const u8));
    if ptr.is_null() {
        panic!("could not load {}", name);
    }
    ptr
}

fn check_glx_extension(glx_exts: &str, ext_name: &str) -> bool {
    for glx_ext in glx_exts.split(" ") {
        if glx_ext == ext_name {
            return true;
        }
    }
    false
}

static mut CTX_ERROR_OCCURED: bool = false;
unsafe extern "C" fn ctx_error_handler(
    _dpy: *mut xlib::Display,
    _ev: *mut xlib::XErrorEvent,
) -> i32 {
    CTX_ERROR_OCCURED = true;
    0
}

unsafe fn check_gl_error() {
    let err = gl::GetError();
    if err != gl::NO_ERROR {
        println!("got gl error {}", err);
    }
}

fn get_glxfbconfig(
    dpy: *mut xlib::Display,
    screen_num: i32,
    visual_attribs: &[i32],
) -> GLXFBConfig {
    unsafe {
        let mut fbcount: c_int = 0;
        let fbcs = glXChooseFBConfig(
            dpy,
            screen_num,
            visual_attribs.as_ptr(),
            &mut fbcount as *mut c_int,
        );

        if fbcount == 0 {
            panic!("could not find compatible fb config");
        }
        // we pick the first from the list
        let fbc = *fbcs;
        xlib::XFree(fbcs as *mut c_void);
        fbc
    }
}

include!("macros.rs");
include!("windowing/xcb.rs");
include!("visuals/style.rs");
include!("visuals/layer.rs");
include!("visuals/sprite.rs");
include!("visuals/anim.rs");
//include!("player/audio2.rs");
include!("player/audio.rs");
include!("player/audio_w_vis.rs");
include!("player/video.rs");
include!("player/player.rs");
include!("rhai.rs");

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut e = Rhai::new(args);
    e.run();
}