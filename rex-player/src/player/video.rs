// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use ffmpeg_next::option::Type::{Duration as FDuration};

pub struct VideoPlaybackThread {
    control_sender: smol::channel::Sender<ControlCommand>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl VideoPlaybackThread {
    pub async fn flush_to_end(&self) {
        while !self.packet_sender.is_empty() {
            smol::Timer::after(std::time::Duration::from_millis(16)).await;
        }
    }
    pub fn start(
        im: &Visual,
        drw: x::Drawable,
        drb: x::Drawable,
        //start_pts: i64,
        settings: StreamSettings,
        stream: &ffmpeg_next::format::stream::Stream,
        sender: smol::channel::Sender<i32>
    ) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::bounded(12);
        let (packet_sender, packet_receiver) = smol::channel::bounded::<ffmpeg_next::codec::packet::packet::Packet>(12);
        let decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        let mut packet_decoder = decoder_context.decoder().video()?;


        let sen2 = sender.clone();
        let mut clock = StreamClock::new(stream,settings.speed_factor);
        let m = im.clone();
        let receiver_thread =
            thread::Builder::new().name("video playback thread".into()).spawn(move|| {
                smol::block_on(async move {
                    let ctx = &CTX;
                    let mut to_rgba_rescaler: Option<Rescaler> = Option::None;

                    let mut to_map: Option<x::Pixmap> = None;
                    let fs: i64 = settings.frame_skip as i64;
                    let packet_receiver_impl = async {
                        let mut d: i64 = 0;
                        loop {
                            smol::future::yield_now().await;
                            if !control_receiver.is_empty() {
                              //  println!("RECV");
                                match control_receiver.recv().await {
                                    Ok(ControlCommand::SkipFwd) => {
                                        while !packet_receiver.is_empty() { packet_receiver.recv().await.unwrap(); }
                                        clock.start_time = Option::None;
                                        println!("VIDEO SKIPPED");
                                    }
                                    _ => {}
                                }
                            }

                            let Ok(packet) = packet_receiver.recv().await else { break };
                            packet_decoder.send_packet(&packet).unwrap_or(());
                            let mut decoded_frame = ffmpeg_next::util::frame::Video::empty();
                            while packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                                let pts = decoded_frame.pts();
                                if let Some(delay) =
                                    clock.convert_pts_to_instant(pts)
                                {
                                   // println!("{:?}", delay);
                                    if delay.as_millis().abs_diff(0) > 250 {
                                        let secs_since_start = Duration::from_secs_f64(pts.unwrap() as f64 * clock.time_base_seconds * clock.speed_factor);
                                        println!("SECS {:?}",secs_since_start);
                                        clock.start_time = Option::from(std::time::Instant::now() - secs_since_start);
                                        continue;
                                        /*
                                        smol::Timer::after(Duration::from_millis(1)).await;*/
                                    } else {
                                        //println!("{:?}",delay);
                                        smol::Timer::after(delay).await;
                                    }
                                    //println!("{:?}",delay);

                                   /* if d > 0 {
                                        let delay2 = delay.as_millis() as i64 - d;
                                        if delay2 > 0 {
                                            println!("!{:?}", delay2);
                                            smol::Timer::after(Duration::from_millis(delay2 as u64)).await;
                                        }
                                    } else {*/

                                    //}
                                }

                                if to_rgba_rescaler.is_none() {
                                    to_rgba_rescaler = Some(rgba_rescaler_for_frame(&decoded_frame, m.width as u32, m.height as u32));
                                }

                                let rescaler = to_rgba_rescaler.as_mut().unwrap();
                                let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                                rescaler.run(&decoded_frame, &mut rgb_frame).unwrap();
                                let data = rgb_frame.data(0);
                                let bytes = data.len();
                                let bf = (bytes / (rgb_frame.width() * 4) as usize) as u16;

                                if to_map.is_none() {
                                    /*let rq = x::CreatePixmap {
                                        depth: ctx.depth,
                                        pid: m.buf,
                                        drawable: drw,
                                        width: m.width,
                                        height: bf
                                    };
                                    ctx.request(&rq);*/
                                    to_map = Some(ctx.new_pixmap(drw,m.width,bf));
                                    println!("to map {:?}",to_map);
                                }
                                let map = to_map.unwrap();
                                let mdrw = Drawable::Pixmap(map);
                                let mbuf = Drawable::Pixmap(m.buf);
                                let mgc = ctx.new_gc(drw, 0xFFFFFFFF, 0x00000000);

                                let yofs = (m.height as i16 - bf as i16) / 2;
                                ctx.fill(mgc, mdrw, data, 0, 0, m.width, bf);
                                ctx.copy(mgc, mdrw, drw, 0, 0, m.x, m.y + yofs, m.width, bf);
                                ctx.copy(mgc, mdrw, drb, 0, 0, m.x, m.y + yofs, m.width, bf);
                                ctx.copy(mgc, mdrw, mbuf, 0, 0, 0, yofs, m.width, bf);
                             //   println!("Blitted");
                            }
                        }
                        //  println!("{fc} frames received");

                      // sender.send(Media::EOF).await.unwrap();
                    }
                        .fuse()
                        .shared();

                        let mut playing = true;
                        loop {
                            let packet_receiver_m: OptionFuture<_> =
                                if playing { Some(packet_receiver_impl.clone()) } else { None }.into();

                            smol::pin!(packet_receiver_m);
                            futures::select! {
                                _ = packet_receiver_m => {},

                             /*   received_command = control_receiver.recv().fuse() => {
                                    match received_command {
                                        Ok(ControlCommand::Pause) => {
                                            playing = false;
                                        }
                                        Ok(ControlCommand::Play) => {
                                            playing = true;
                                        }
                                        Ok(ControlCommand::SkipFwd) => {
                                           // clock = StreamClock::new(stream,settings.speed_factor);
                                            //while !packet_receiver.is_empty() { packet_receiver.recv().await.unwrap(); }
                                        }
                                        _ => {
                                            if received_command.is_err() {
                                         //       sender.send(Media::EOF).await.unwrap();
                                                return;
                                            }
                                        }
                                    }
                                }*/
                            }
                        }
                });
                //sen2.send_blocking(Media::EOF).unwrap();
            })?;

        Ok(Self { control_sender, packet_sender, receiver_thread: Some(receiver_thread) })
    }

    pub async fn receive_packet(&self, packet: ffmpeg_next::codec::packet::packet::Packet) -> bool {
        match self.packet_sender.send(packet).await {
            Ok(_) => return true,
            Err(smol::channel::SendError(_)) => return false
        }
    }

    pub async fn send_control_message(&self, message: ControlCommand) {
        self.control_sender.send(message).await.unwrap();
    }
}

impl Drop for VideoPlaybackThread {
    fn drop(&mut self) {
        self.control_sender.close();
        if let Some(receiver_join_handle) = self.receiver_thread.take() {
            receiver_join_handle.join().unwrap();
        }
    }
}

#[derive(Debug)]
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

    fn convert_pts_to_instant(&mut self,lpts: Option<i64>) -> Option<std::time::Duration> {
        lpts.and_then(|lpts|{
            let pts = (lpts as f64 * self.speed_factor) as i64;
            let secs_since_start = Duration::from_secs_f64(pts as f64 * self.time_base_seconds);
            if self.start_time.is_none() { self.start_time = Option::from(std::time::Instant::now() - secs_since_start); }
        self.start_time.unwrap().checked_add(secs_since_start)}).map(|absolute_pts| absolute_pts.duration_since(std::time::Instant::now()))
    }
}
