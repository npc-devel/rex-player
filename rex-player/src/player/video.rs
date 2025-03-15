// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use ffmpeg_next::option::Type::{Duration as FDuration};

pub struct VideoPlaybackThread {
    control_sender: smol::channel::Sender<(i64,String)>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl VideoPlaybackThread {
    pub fn flush_to_end(&self) {
        while !self.packet_sender.is_empty() {
            thread::sleep(Duration::from_millis(1));
        }
    }
    pub fn start(
        im: &Visual,
        drw: x::Drawable,
        drb: x::Drawable,
        //start_pts: i64,
        settings: StreamSettings,
        stream: &ffmpeg_next::format::stream::Stream,
        sender: smol::channel::Sender<(i64,String)>
    ) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::bounded::<(i64,String)>(128);
        let (packet_sender, packet_receiver) = smol::channel::bounded::<ffmpeg_next::codec::packet::packet::Packet>(50);
        let decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        let mut packet_decoder = decoder_context.decoder().video()?;


        let sen2 = sender.clone();
        let mut clock = StreamClock::new(stream,settings.speed_factor);
        let m = im.clone();
        let receiver_thread =
            thread::Builder::new().name("video playback thread".into()).spawn(move|| {
                let mut silent = false;
                smol::block_on(async move {
                    let ctx = &CTX;
                    let mut to_rgba_rescaler: Option<Rescaler> = Option::None;

                    let mut to_map: Option<x::Pixmap> = None;
                    let fs: i64 = settings.frame_skip as i64;
                    let packet_receiver_impl = async {
                        let mut key_pts = 0;
                        let mut find_key = false;
                        let mut seek_fwd = false;

                        let mgc = ctx.new_gc(drw, 0xFFFFFFFF, 0x00000000);
                        let mbuf = Drawable::Window(m.window);

                        let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                        loop {
                            smol::future::yield_now().await;
                            loop {
                                let packetr = packet_receiver.recv().await;
                                if !packetr.is_ok() { continue };
                                let mut packet = packetr.ok().unwrap();
                                if !find_key || packet.is_key() {
                                    if find_key {
                                        key_pts = packet.pts().unwrap();
                                        find_key = false;
                                    }
                                    packet_decoder.send_packet(&packet).unwrap_or(());
                                    break;
                                }
                            }
                            let mut decoded_frame = ffmpeg_next::util::frame::Video::empty();
                            let mut shot = false;
                            let mut fdel = Duration::from_millis(2);
                            let mut die = false;

                            while packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                                if seek_fwd && decoded_frame.pts().unwrap() < key_pts { continue }

                                shot = true;

                                if to_rgba_rescaler.is_none() {
                                    to_rgba_rescaler = Some(rgba_rescaler_for_frame(&decoded_frame, m.width as u32, m.height as u32));
                                }

                                let rescaler = to_rgba_rescaler.as_mut().unwrap();
                                rescaler.run(&decoded_frame, &mut rgb_frame).unwrap();

                                let data = rgb_frame.data(0);
                                let bytes = data.len();
                                let bf = (bytes / (rgb_frame.width() * 4) as usize) as u16;

                                if to_map.is_none() {
                                    to_map = Some(ctx.new_pixmap(drw, m.width, bf));
                                }
                                let map = to_map.unwrap();
                                let mdrw = Drawable::Pixmap(map);

                                let pts = decoded_frame.pts();
                                if let Some(delay) =
                                    clock.convert_pts_to_instant(pts,key_pts)
                                {
                                    let delay_ms = delay.as_millis();
                                    if delay_ms>=0 && delay_ms<50  {
                                        fdel = delay;
                                        if key_pts > 0 {
                                            key_pts = 0;
                                            let dof = Duration::from_secs_f64(0.005);
                                            if fdel > dof {
                                                fdel = fdel.checked_sub(dof).unwrap_or(dof);
                                                clock.start_time = Option::from(clock.start_time.unwrap().checked_sub(dof).unwrap_or(clock.start_time.unwrap()));
                                            }
                                        }
                                    }
                                    else {
                                        clock.start_time = Option::None;
                                    }
                                }

                                loop {
                                    futures::select! {
                                        _ = futures::FutureExt::fuse(smol::Timer::after(fdel)) => {}
                                        cmd = control_receiver.recv().fuse() => {
                                             let cr = cmd.unwrap_or((Player::CTL_NONE,"".to_string()));
                                             match cr.0 {
                                                Player::CTL_VIDEO_DIE_SILENT=> {
                                                    die = true;
                                                    silent = true;
                                                    break;
                                                }
                                                Player::CTL_VIDEO_DIE=> {
                                                    die = true;
                                                    break;
                                                }
                                                Player::CTL_SEEK_ABS|Player::CTL_SEEK_REL => {
                                                    clock.start_time = Option::None;
                                                    find_key = true;
                                                    seek_fwd = cr.1!="r";
                                                    while !control_receiver.is_empty() { control_receiver.recv().await.unwrap_or_default(); }
                                                    break;
                                                }
                                                _ => { break }
                                            }
                                        }
                                    }
                                    break;
                                }
                                if die { break }
                                if clock.start_time.is_some() {
                                    let yofs = (m.height as i16 - bf as i16) / 2;
                                    ctx.fill(mgc, mdrw, data, 0, 0, m.width, bf);
                                    ctx.copy(mgc, mdrw, drb, 0, 0, m.x, m.y + yofs, m.width, bf);
                                    ctx.copy(mgc, mdrw, mbuf, 0, 0, 0, yofs, m.width, bf);
                                } else { break }
                            }

                            if !shot {
                                futures::select! {
                                    _ = futures::FutureExt::fuse(smol::Timer::after(Duration::from_millis(10))) => {}
                                    cmd = control_receiver.recv().fuse() => {
                                         let cr = cmd.unwrap();
                                         match cr.0 {
                                            Player::CTL_VIDEO_DIE_SILENT=> {
                                                silent = true;
                                                die = true;
                                                break;
                                            }
                                            Player::CTL_VIDEO_DIE=> {
                                                die = true;
                                                break;
                                            }
                                            Player::CTL_SEEK_ABS|Player::CTL_SEEK_REL => {
                                              //  println!("VIDEO FLUSHING");
                                                //let wdelay = Duration::from_millis(10);

                                                clock.start_time = Option::None;
                                                find_key = true;
                                                seek_fwd = cr.1!="r";
                                                while !control_receiver.is_empty() { control_receiver.recv().await.unwrap_or_default(); }
                                                //break;
                                                //println!("VIDEO FLUSHED");
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            if die {
                                break
                            }
                        }
                        ctx.drop_gc(mgc);
                        if to_map.is_some() { ctx.drop_pixmap(to_map.unwrap()) }
                        let gc = ctx.new_gc(drb,0,0);
                        ctx.rect(gc,drb,m.x, m.y, m.width, m.height);
                        ctx.rect(gc,mbuf,0,0, m.width, m.height);
                        ctx.drop_gc(gc);
                        silent
                    }
                    .fuse()
                    .shared();

                    let mut playing = true;
                    loop {
                        let packet_receiver_m: OptionFuture<_> =
                            if playing { Some(packet_receiver_impl.clone()) } else { None }.into();

                        smol::pin!(packet_receiver_m);
                        futures::select! {
                            silent = packet_receiver_m => {
                                println!("Video silence: {}",silent.unwrap());
                                if !silent.unwrap() { sender.send((Media::EOF,"".to_string())).await.unwrap_or_default() }
                                break;
                            }
                        }
                    }
                });
                sen2.send_blocking((Player::VIDEO_DEAD,"".to_string())).unwrap_or(());
            })?;

        Ok(Self { control_sender, packet_sender, receiver_thread: Some(receiver_thread) })
    }

    pub async fn receive_packet(&self, packet: ffmpeg_next::codec::packet::packet::Packet) -> bool {
        match self.packet_sender.send(packet).await {
            Ok(_) => return true,
            Err(smol::channel::SendError(_)) => return false
        }
    }

    pub async fn send_control_message(&self, message: i64) {
        self.control_sender.send((message,"".to_string())).await.unwrap_or_default();
    }

    pub async fn send_control_message_ex(&self, message: i64,ex:&str) {
        self.control_sender.send((message,ex.to_string())).await.unwrap_or_default();
    }
}

impl Drop for VideoPlaybackThread {
    fn drop(&mut self) {
  //      println!("DROPPING VIDEO");

        //self.send_ctl(Player::CTL_VIDEO_DIE);
    //    self.control_sender.close();
        //if let Some(receiver_join_handle) = self.receiver_thread.take() {
          //  receiver_join_handle.join().unwrap_or(());
        //}
    }
}

#[derive(Clone)]
struct StreamClock {
    speed_factor: f64,
    time_base_seconds: f64,
    start_time: Option<std::time::Instant>
}

impl StreamClock {
    fn new(stream: &ffmpeg_next::format::stream::Stream,speed_factor: f64) -> Self {
        let time_base_seconds = stream.time_base();

        let time_base_seconds =
            time_base_seconds.numerator() as f64 / time_base_seconds.denominator() as f64;

        Self { speed_factor,time_base_seconds, start_time: Option::None }
    }

    fn convert_pts_to_instant(&mut self,lpts: Option<i64>,mut origin: i64) -> Option<std::time::Duration> {
        lpts.and_then(|lpts|{
            let mut nsf = self.speed_factor + 0.00;
            if nsf < 0.0 { nsf = 0.0; }
            let pts = lpts;
            let ofs = 0.0;
            origin -= (0.25/self.time_base_seconds) as i64;
            if origin<0 { origin = 0 }

            let mut secs = pts as f64 * self.time_base_seconds;
            let mut secsa = (pts-origin) as f64 * self.time_base_seconds;
         //   let secs_since_start_r = Duration::from_secs_f64(secs*self.speed_factor);
            secs *= nsf;
            secs += ofs;
            if secs < 0.0 { secs = 0.0 }
            if secsa < 0.0 { secsa = 0.0 }
            let secs_since_start = Duration::from_secs_f64(secs);
            let secs_since_start_a = Duration::from_secs_f64(secsa);
            if self.start_time.is_none() { self.start_time = Option::from(std::time::Instant::now() - secs_since_start); }
            self.start_time.unwrap().checked_add(secs_since_start)
        }).map(|absolute_pts| absolute_pts.duration_since(std::time::Instant::now()))
    }
}
