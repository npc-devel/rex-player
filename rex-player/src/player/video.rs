// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use ffmpeg_next::option::Type::Duration as FDuration;

pub struct VideoPlaybackThread {
    control_sender: smol::channel::Sender<ControlCommand>,
    packet_sender: smol::channel::Sender<ffmpeg_next::codec::packet::packet::Packet>,
    receiver_thread: Option<std::thread::JoinHandle<()>>,
}

impl VideoPlaybackThread {
    pub fn start(
        start_pts: i64,
        settings: StreamSettings,
        stream: &ffmpeg_next::format::stream::Stream,
        mut video_frame_callback: Box<dyn FnMut(&ffmpeg_next::util::frame::Video) + Send>,
        sender: smol::channel::Sender<i32>
    ) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::unbounded();

        let (packet_sender, packet_receiver) = smol::channel::bounded(128);

        let decoder_context = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?;
        let mut packet_decoder = decoder_context.decoder().video()?;


        let mut clock = StreamClock::new(start_pts,stream);
        let sen2 = sender.clone();

        let receiver_thread =
            std::thread::Builder::new().name("video playback thread".into()).spawn(move || {
                smol::block_on(async move {
                    let packet_receiver_impl = async {
                        loop {
                            let Ok(packet) = packet_receiver.recv().await else { break };
                            smol::future::yield_now().await;
                            packet_decoder.send_packet(&packet).unwrap();

                            let mut decoded_frame = ffmpeg_next::util::frame::Video::empty();
                    //        let mut fc = 0;
                            while packet_decoder.receive_frame(&mut decoded_frame).is_ok() {
                                let pts = decoded_frame.pts();
                                if pts.unwrap_or(0) == 0 { continue }

                                if let Some(delay) =
                                    clock.convert_pts_to_instant(settings.speed_factor,pts)
                                {
                                    if delay.is_zero() { smol::Timer::after(std::time::Duration::from_nanos(100)).await; }
                                    else {
                                        smol::Timer::after(delay).await;
                                    }
                                }
                                //fc+=1;
                                video_frame_callback(&decoded_frame);
                            }
                          //  println!("{fc} frames received");
                        }
                        sender.send(Media::EOF).await.unwrap();
                    }
                    .fuse()
                    .shared();

                    let mut playing = true;
                    loop {
                        let packet_receiver: OptionFuture<_> =
                            if playing { Some(packet_receiver_impl.clone()) } else { None }.into();

                        smol::pin!(packet_receiver);
                        futures::select! {
                            _ = packet_receiver => {},

                            received_command = control_receiver.recv().fuse() => {
                                match received_command {
                                    Ok(ControlCommand::Pause) => {
                                        playing = false;
                                    }
                                    Ok(ControlCommand::Play) => {
                                        playing = true;
                                    }
                                    Err(_) => {
                                        sender.send(Media::EOF).await.unwrap();
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    //sender.send(Media::EOF).await.unwrap();
                });
                sen2.send_blocking(Media::EOF).unwrap();
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

struct StreamClock {
    start_pts: i64,
    time_base_seconds: f64,
    start_time: Option<std::time::Instant>
}

impl StreamClock {
    fn new(start_pts: i64,stream: &ffmpeg_next::format::stream::Stream) -> Self {
        let time_base_seconds = stream.time_base();
        //println!("{:?}",time_base_seconds);

        let time_base_seconds =
            time_base_seconds.numerator() as f64 / time_base_seconds.denominator() as f64;

        //let start_time = std::time::Instant::now();// + std::time::Duration::from_secs((start_pts as f64 * time_base_seconds) as u64);

        Self { start_pts, time_base_seconds, start_time: Option::None }
    }

    fn convert_pts_to_instant(&mut self,speed_factor: f64, mut pts: Option<i64>) -> Option<std::time::Duration> {
            pts.and_then(|lpts| {
                let pts = (lpts as f64 * speed_factor) as i64;
                let secs_since_start = Duration::from_secs_f64(pts as f64 * self.time_base_seconds);
                if self.start_time.is_none() { self.start_time = Option::from(std::time::Instant::now() - secs_since_start); }
                self.start_time.unwrap().checked_add(secs_since_start)

            })
                .map(|absolute_pts| absolute_pts.duration_since(std::time::Instant::now()))

    }
}
