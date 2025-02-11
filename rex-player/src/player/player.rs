// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT


use std::path::PathBuf;
use ffmpeg_next::{rescale, Rescale};
use futures::TryFutureExt;
use smol::stream::StreamExt;

#[derive(Clone, Copy)]
pub enum ControlCommand {
    Play,
    Pause,
    SkipFwd
}

pub struct Player {
    control_sender: smol::channel::Sender<ControlCommand>,
    demuxer_thread: Option<std::thread::JoinHandle<()>>
}

impl Player {
    const HAS_AUDIO: i32 = 1;
    const HAS_VIDEO: i32 = 2;
    pub fn check(path: PathBuf)->(i32,Option<Input>) {
        let mut ret: i32 = 0;
        let o_input_context = ffmpeg_next::format::input(&path);
        if o_input_context.is_ok() {
            let input_context = o_input_context.unwrap();
            if input_context.streams().best(ffmpeg_next::media::Type::Audio).is_some() { ret = ret | Self::HAS_AUDIO }
            if input_context.streams().best(ffmpeg_next::media::Type::Video).is_some() { ret = ret | Self::HAS_VIDEO }
            (ret,Option::from(input_context))
        } else {
            (ret, Option::None)
        }
    }

    pub fn start(
        im: &Visual,
        drw: x::Drawable,
        drb: x::Drawable,
        mut input_context: Input,
        settings: StreamSettings,
        sender: smol::channel::Sender<i32>
    ) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::unbounded();
        let sen2 = sender.clone();
        let con2 = control_receiver.clone();
        let fs = settings.frame_skip as i64;



        let m = im.clone();
        let demuxer_thread = std::thread::Builder::new().name("demuxer thread".into()).spawn(move || {
            smol::block_on(async move {
                let ctx = &CTX;

                sender.send(Media::LOADED).await.unwrap_or(());
                let mut start_pts: i64 = 0;
                let mut start_secs: i64 = 0;
                let mut end_pts: i64 = input_context.duration();
                if settings.start_secs != 0.0 {
                    start_secs = settings.start_secs as i64;
                    if start_secs < 0 {
                        start_secs =  rescale::Rescale::rescale(&end_pts,rescale::TIME_BASE, (1, 1)) + start_secs;
                    }
                    start_pts = rescale::Rescale::rescale(&start_secs, (1, 1), rescale::TIME_BASE);

                    input_context.seek(start_pts, ..start_pts).unwrap();
                    end_pts -= start_pts;
                }

                let mut video_stream = input_context.streams().best(ffmpeg_next::media::Type::Video).unwrap();
                let video_stream_index = video_stream.index();

                let video_playback_thread = VideoPlaybackThread::start(&m, drw, drb, settings.clone(), &video_stream, sender.clone()).unwrap();

                let packet_forwarder_impl = async {
                    if settings.use_audio {
                        let mut d = 0;
                        let mut so: i64 = start_secs;

                        let audio_stream = input_context.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
                        let audio_stream_index = audio_stream.index();
                        let audio_playback_thread_r = AudioPlaybackThread::start(&audio_stream);

                        if audio_playback_thread_r.is_ok() {
                            let audio_playback_thread = audio_playback_thread_r.unwrap();

                            loop {
                                if d > 0 {
                                    let t = rescale::Rescale::rescale(&d, rescale::TIME_BASE, (1, 1));
                                    println!("SEEK {d} {t}");
                                    input_context.seek(d, ..d).unwrap();
                                    video_playback_thread.send_control_message(ControlCommand::SkipFwd).await;
                                    d = 0;
                                }
                                for (stream, packet) in input_context.packets() {
                                    if !control_receiver.is_empty() {
                                        println!("RECV");
                                        let command = control_receiver.recv().fuse().await;
                                        match command {
                                            Ok(ControlCommand::SkipFwd) => {
                                                so += 60;
                                                d = packet.pts().unwrap() + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                println!("TRY SEEK {d}");
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                    if stream.index() == audio_stream_index {
                                        audio_playback_thread.receive_packet(packet).await;
                                    } else if stream.index() == video_stream_index {
                                        video_playback_thread.receive_packet(packet).await;
                                    }
                                }
                                if d == 0 { break }
                            }
                        }
                    }

                    let mut d = 0;
                    let mut so: i64 = start_secs;
                    loop {
                        if d > 0 {
                            let t = rescale::Rescale::rescale(&d,rescale::TIME_BASE, (1, 1));
                            println!("SEEK {d} {t}");
                            input_context.seek(d , ..d).unwrap();
                            video_playback_thread.send_control_message(ControlCommand::SkipFwd).await;
                            d = 0;
                        }
                        for (stream, packet) in input_context.packets() {
                            if !control_receiver.is_empty() {
                                println!("RECV");
                                let command = control_receiver.recv().fuse().await;
                                match command {
                                    Ok(ControlCommand::SkipFwd) => {
                                        so += 60;
                                        d = packet.pts().unwrap() + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                        println!("TRY SEEK {d}");
                                        break;
                                    }
                                    _ => {}
                                }
                            }

                            if stream.index() == video_stream_index {
                                let pts = packet.pts().unwrap();
                                video_playback_thread.receive_packet(packet).await;
                            }
                        }
                        if d == 0 { break }
                    }
                }.fuse().shared();

                let mut playing = true;
                let mut eof = false;
                loop {
                    let packet_forwarder: OptionFuture<_> = if playing { Some(packet_forwarder_impl.clone()) } else { None }.into();
                    smol::pin!(packet_forwarder);
                    futures::select! {
                        pfr = packet_forwarder => {
                            video_playback_thread.flush_to_end().await;
                            eof = true;
                            sen2.send(Media::EOF).await.unwrap_or(());
                        }
                        /*received_command = control_receiver.recv().fuse() => {
                            match received_command {
                                Ok(command) => {
                                    video_playback_thread.send_control_message(command).await;
                                    //audio_playback_thread.send_control_message(command).await;
                                    match command {
                                        ControlCommand::Play => {
                                            // Continue in the loop, polling the packet forwarder future to forward
                                            // packets
                                            playing = true;
                                        },
                                        ControlCommand::Pause => {
                                            playing = false;
                                        },
                                        ControlCommand::SkipFwd => {
                                        //    let p = 0;
                                            //sen2.send(Media::POS_START + p).await.unwrap();
                                          //  return;
                                        },
                                        _ => {}
                                    }
                                }
                                Err(_) => {
                                 //   sen2.send(Media::EOF).await.unwrap();
                                    return;
                                }
                            }
                        }*/
                    }
                //    if eof { break }
                }

            });
        })?;

        Ok(Self {
            control_sender,
            demuxer_thread: Some(demuxer_thread)
        })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.control_sender.close();
        //if let Some(decoder_thread) = self.demuxer_thread.take() {
          //  decoder_thread.thread();
        //}
    }
}
