
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
use ffmpeg::software::scaling::{context::Context as VideoContext, flag::Flags};
use ffmpeg::software::resampling::{context::Context as AudioContext};
use ffmpeg::util::frame::video::Video;
use ffmpeg::util::frame::audio::Audio;
use ffmpeg_next::codec::profile::JPEG2000::CStreamNoRestriction;
use ffmpeg_next::decoder::video;
use ffmpeg_next::{software, Error};
use ffmpeg_next::format::context::input;
use ffmpeg_next::format::context::Input;

use image::{DynamicImage, Rgba, ImageReader, EncodableLayout};
use image::GenericImageView;
use image::imageops::FilterType;

use rhai::{Engine, EvalAltResult, Scope as RScope, CustomType, TypeBuilder, INT};
use rhai::packages::Package;
use rhai_fs::FilesystemPackage;
use rhai_rand::RandomPackage;

use json::{array, JsonValue};
use rand::{random, thread_rng, Rng};

include!("macros.rs");
include!("rhai.rs");
include!("windowing/xcb.rs");
include!("visuals/style.rs");
include!("visuals/layer.rs");
include!("visuals/sprite.rs");
include!("player/audio.rs");
include!("player/video.rs");
include!("player/player.rs");

fn main() {
    let mut e = Rhai::new();
    e.run();
}