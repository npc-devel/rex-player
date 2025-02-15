// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT


use std::path::PathBuf;
use ffmpeg_next::{rescale, Rescale};
use futures::TryFutureExt;
use smol::stream::StreamExt;

pub struct Player {
    control_sender: smol::channel::Sender<i64>,
    demuxer_thread: Option<std::thread::JoinHandle<()>>
}

impl Player {
    const HAS_AUDIO: i32 = 1;
    const HAS_VIDEO: i32 = 2;

    /*const Play:i64 = 1;
    const Pause:i64 = 1;*/
    /*const SkipFwd:i64 = 1;
    const SkipBkw:i64 = 1;
    const SeekFwd:i64 = 1;
    const C:i64 = 1;*/
    const CTL_DIE:i64 = -1;
    const CTL_SEEK_ABS_MIN:i64 = 1000+36000;
    const CTL_SEEK_ABS:i64 = 1000+36000*2;
    const CTL_SEEK_ABS_MAX:i64 = 1000+36000*3;
    const CTL_SEEK_REL_MIN:i64 = 1000+36000*4;
    const CTL_SEEK_REL:i64 = 1000+36000*5;
    const CTL_SEEK_REL_MAX:i64 = 1000+36000*6;
    const CTL_NONE:i64 = i64::MAX;
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

    pub fn pts_delta(mut so:i64,mut d:i64,start_pts:i64,end_pts:i64)->i64 {
        let pts_edge = rescale::Rescale::rescale( &d.abs(),(1,1),rescale::TIME_BASE)*4;
        let mso = rescale::Rescale::rescale(&(end_pts-40), rescale::TIME_BASE, (1, 1)) - d.abs()/2;
        let fwd = d > 0;
        if d > 0 {
            if pts_edge > (end_pts-start_pts) {
                d = rescale::Rescale::rescale(&((end_pts-start_pts)/2), rescale::TIME_BASE,(1,1));
                println!("Slo {d}");
                if (end_pts-start_pts)/2 > (end_pts-pts_edge/2) {
                    d = 0;
                }
            }
        } /*else {
            if start_pts < pts_edge {
                d = rescale::Rescale::rescale(&((end_pts - start_pts) / 2), rescale::TIME_BASE, (1, 1));
                println!("Slo {d}");
                if start_pts < pts_edge/2 {
                    d = 0;
                }
            }
            so = so + d;
        }*/
        so = so + d;

        println!("{start_pts}-{end_pts}-{pts_edge}={so}/{mso}");
        let eds = d.abs()/2;
        if fwd  {
            if so < (mso-eds) {
                so
            } else {
                mso-eds
            }
        } else {
            if so>0 { so } else { 0 }
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
        let (control_sender, control_receiver) = smol::channel::bounded(10);
        //let sen2 = sender.clone();
       // let con2 = control_receiver.clone();
        let fs = settings.frame_skip as i64;

        let m = im.clone();
        let demuxer_thread = std::thread::Builder::new().name("demuxer thread".into()).spawn(move || {
            smol::block_on(async move {
                let ctx = &CTX;

                sender.send(Media::LOADED).await.unwrap_or(());
                let mut start_pts: i64 = 0;
                let mut start_secs: i64 = 0;
                let mut end_pts: i64 = input_context.duration();
                let end_secs: i64 = rescale::Rescale::rescale(&end_pts,rescale::TIME_BASE, (1, 1));
                if settings.start_secs != 0.0 {
                    start_secs = settings.start_secs as i64;
                    if start_secs < 0 {
                        start_secs = end_secs + start_secs;
                    }
                    start_pts = rescale::Rescale::rescale(&start_secs, (1, 1), rescale::TIME_BASE);

                    input_context.seek(start_pts, ..start_pts).unwrap();
                    end_pts -= start_pts;
                }

                let mut o_video_playback_thread: Option<VideoPlaybackThread> = Option::None;
                let packet_forwarder_impl = async {
                    if settings.use_audio {
                        let mut d = i64::MIN;
                        let mut so: i64 = start_secs;
                        let mut chain_cmd = Self::CTL_NONE;

                        let audio_stream = input_context.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
                        let audio_stream_index = audio_stream.index();
                        let audio_playback_thread_r = AudioPlaybackThread::start(&audio_stream);

                        let vso = input_context.streams().best(ffmpeg_next::media::Type::Video);
                        if audio_playback_thread_r.is_ok() && vso.is_some() {
                            let mut video_stream = vso.unwrap();
                            let video_stream_index = video_stream.index();

                            o_video_playback_thread = Option::from(VideoPlaybackThread::start(&m, drw, drb, settings.clone(), &video_stream, sender.clone()).unwrap());
                            let audio_playback_thread = audio_playback_thread_r.unwrap();
                            let video_playback_thread = o_video_playback_thread.unwrap();
                            loop {
                                if d != i64::MIN {
                                   // let t = rescale::Rescale::rescale(&d, rescale::TIME_BASE, (1, 1));
                              //      println!("SEEK {d} {t}");
                                    start_pts += d;
                                    input_context.seek(start_pts, ..start_pts).unwrap_or(());
                                    video_playback_thread.send_control_message(chain_cmd).await;
                                    audio_playback_thread.send_control_message(chain_cmd).await;
                                 /*   for (stream, packet) in input_context.packets() {
                                        if stream.index() == video_stream_index {
                                            if packet.is_key() {
                                                let p = packet.pts().unwrap();
                                                println!("SEEK TO {p}");
                                                start_pts = p + d - 3;
                                                input_context.seek(start_pts, ..start_pts).unwrap();
                                                break;
                                            }
                                        }
                                    }*/
                                    d = i64::MIN;
                               //     video_playback_thread.send_control_message(ControlCommand::SkipFwd).await;
                                }
                                for (stream, packet) in input_context.packets() {

                                    if stream.index() == audio_stream_index {
                                        audio_playback_thread.receive_packet(packet).await;
                                    } else if stream.index() == video_stream_index {
                                        if !control_receiver.is_empty() {
                                            println!("RECV");
                                            let command = control_receiver.recv().fuse().await;
                                            match command {
                                                Ok(Self::CTL_DIE) => {
                                                    audio_playback_thread.send_control_message(Self::CTL_DIE).await;
                                                    video_playback_thread.send_control_message(Self::CTL_DIE).await;
                                                    return;
                                                }
                                                /*Ok(Self::SEEK_ABS) => {
                                                    start_pts = packet.pts().unwrap();
                                                    so = Self::pts_delta(so,30,start_pts,end_pts);
                                                    d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    chain_cmd = command.ok().unwrap();
                                                    println!("TRY SEEK {d}");
                                                    break;
                                                }
                                                Ok(ControlCommand::SkipBkw) => {
                                                    start_pts = packet.pts().unwrap();
                                                    so = Self::pts_delta(so,-30, start_pts,end_pts);
                                                    d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    chain_cmd = command.ok().unwrap();
                                                    println!("TRY SEEK {d}");
                                                    break;
                                                }
                                                Ok(ControlCommand::SeekFwd) => {
                                                    start_pts = packet.pts().unwrap();
                                                    so = Self::pts_delta(so, 10,start_pts,end_pts);
                                                    d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    chain_cmd = command.ok().unwrap();
                                                    println!("TRY SEEK {d}");
                                                    break;
                                                }
                                                Ok(ControlCommand::SeekBkw) => {
                                                    start_pts = packet.pts().unwrap();
                                                    so = Self::pts_delta(so, -10,start_pts,end_pts);
                                                    d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    chain_cmd = command.ok().unwrap();
                                                    println!("TRY SEEK {d}");
                                                    break;
                                                }*/
                                                _ => {
                                                    if command.is_ok() {
                                                        let cv = command.unwrap();
                                                        if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                            let sd = cv - Self::CTL_SEEK_ABS;
                                                            if sd > 0 {
                                                                start_pts = 0;
                                                                so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                                d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            } else {
                                                                start_pts = end_pts;
                                                                so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                                d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                                chain_cmd = command.ok().unwrap();

                                                            }
                                                            chain_cmd = Self::CTL_SEEK_ABS;

                                                            println!("TRY SEEK {d}");
                                                            break;
                                                        } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                            let sd = cv - Self::CTL_SEEK_REL;
                                                            start_pts = packet.pts().unwrap();
                                                            so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            chain_cmd = Self::CTL_SEEK_REL;

                                                            println!("TRY SEEK {d}");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        video_playback_thread.receive_packet(packet).await;
                                    }
                                }
                                if d == i64::MIN { break }
                            }
                            video_playback_thread.send_control_message(Self::CTL_DIE).await;
                            audio_playback_thread.send_control_message(Self::CTL_DIE).await;
                        } else if audio_playback_thread_r.is_ok() {
                            let audio_playback_thread = audio_playback_thread_r.unwrap();

                            loop {
                                if d != i64::MIN {
                                    start_pts += d;
                                    input_context.seek(start_pts, ..start_pts).unwrap_or(());
                                    //audio_playback_thread.send_control_message(chain_cmd).await;
                                    /*   for (stream, packet) in input_context.packets() {
                                           if stream.index() == video_stream_index {
                                               if packet.is_key() {
                                                   let p = packet.pts().unwrap();
                                                   println!("SEEK TO {p}");
                                                   start_pts = p + d - 3;
                                                   input_context.seek(start_pts, ..start_pts).unwrap();
                                                   break;
                                               }
                                           }
                                       }*/
                                //    d = i64::MIN;
                                    //     video_playback_thread.send_control_message(ControlCommand::SkipFwd).await;
                                }
                                for (stream, packet) in input_context.packets() {
                                    if stream.index() == audio_stream_index {
                                        if !control_receiver.is_empty() {
                                            println!("RECV");
                                            let command = control_receiver.recv().fuse().await;
                                            match command {
                                                Ok(Self::CTL_DIE) => {
                                                    audio_playback_thread.send_control_message(command.unwrap()).await;
                                                    return;
                                                }
                                                _ => {
                                                    if command.is_ok() {
                                                        let cv = command.unwrap();
                                                        if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                            let sd = cv - Self::CTL_SEEK_ABS;
                                                            if sd > 0 {
                                                                start_pts = 0;
                                                                so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                                d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            } else {
                                                                start_pts = end_pts;
                                                                so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                                d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                                chain_cmd = command.ok().unwrap();

                                                            }
                                                            chain_cmd = Self::CTL_SEEK_ABS;

                                                            println!("TRY SEEK {d}");
                                                            break;
                                                        } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                            let sd = cv - Self::CTL_SEEK_REL;
                                                            start_pts = packet.pts().unwrap();
                                                            so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            chain_cmd = Self::CTL_SEEK_REL;

                                                            println!("TRY SEEK {d}");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        audio_playback_thread.receive_packet(packet).await;
                                    }
                                }
                                if d == i64::MIN { break }
                            }
                            audio_playback_thread.send_control_message(Self::CTL_DIE).await;
                        }
                    }

                    let mut d = i64::MIN;
                    let mut so: i64 = start_secs;
                    let mut chain_cmd = Self::CTL_NONE;
                    let mut video_stream = input_context.streams().best(ffmpeg_next::media::Type::Video).unwrap();
                    let video_stream_index = video_stream.index();

                    let video_playback_thread = VideoPlaybackThread::start(&m, drw, drb, settings.clone(), &video_stream, sender.clone()).unwrap();
                    loop {
                        if d != i64::MIN {
                            //let t = rescale::Rescale::rescale(&d,rescale::TIME_BASE, (1, 1));

                            if end_pts > d {
                                println!("SEEK {d}");
                                start_pts = d;
                                input_context.seek(start_pts, ..start_pts).unwrap_or(());
                                println!("FLUSHING");
                                video_playback_thread.send_control_message(chain_cmd).await;
                                println!("FLUSHED");
                            }
                            d = i64::MIN;
                        }

                        let pts_edge = rescale::Rescale::rescale( &30,(1,1), rescale::TIME_BASE);
                        for (stream, packet) in input_context.packets() {
                            if stream.index() == video_stream_index {
                                let ppts = packet.pts().unwrap();
                                if !control_receiver.is_empty() && ppts > 0 {
                                    println!("RECV");
                                    let command = control_receiver.recv().fuse().await;
                                    match command {
                                        Ok(Self::CTL_DIE) => {
                                            video_playback_thread.send_control_message(Self::CTL_DIE).await;
                                        }/*
                                        Ok(ControlCommand::SkipFwd) => {
                                            start_pts = ppts;
                                            so = Self::pts_delta(so,30,start_pts,end_pts);
                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                            if d != start_pts {
                                                chain_cmd = command.ok().unwrap();
                                                println!("TRY SEEK {d}");
                                            }
                                            break;
                                        }
                                        Ok(ControlCommand::SkipBkw) => {
                                            start_pts = ppts;
                                            so = Self::pts_delta(so,-30,start_pts,end_pts);
                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                            if d != start_pts {
                                                chain_cmd = command.ok().unwrap();
                                                println!("TRY SEEK {d}");
                                            }
                                            break;
                                        }
                                        Ok(ControlCommand::SeekFwd) => {
                                            start_pts = ppts;
                                            so = Self::pts_delta(so,20,start_pts,end_pts);
                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                            if d != start_pts {
                                                chain_cmd = command.ok().unwrap();
                                                println!("TRY SEEK {d}");
                                                break;
                                            }

                                        }
                                        Ok(ControlCommand::SeekBkw) => {
                                            start_pts = ppts;
                                            so = Self::pts_delta(so,-20,start_pts,end_pts);
                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                            if d != start_pts {
                                                chain_cmd = command.ok().unwrap();
                                                println!("TRY SEEK {d}");
                                                break;
                                            }
                                        }*/
                                        _ => {
                                            if command.is_ok() {
                                                let cv = command.unwrap() ;
                                                if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                    if cv > Self::CTL_SEEK_ABS {
                                                        so = Self::pts_delta(0, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                        d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    } else {
                                                        so = Self::pts_delta(end_secs,cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                        d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    }
                                                    chain_cmd = Self::CTL_SEEK_ABS;

                                                    println!("TRY ABS SEEK {d}");
                                                    break;
                                                } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                    let sd = cv - Self::CTL_SEEK_REL;
                                                    start_pts = packet.pts().unwrap();
                                                    so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                    d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    chain_cmd = Self::CTL_SEEK_REL;

                                                    println!("TRY SEEK {d}");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                video_playback_thread.receive_packet(packet).await;
                            }
                        }
                        if d == i64::MIN { break }
                    }
                    video_playback_thread.send_control_message(Self::CTL_DIE).await;
                }.fuse().shared();

                let mut playing = true;
                let mut eof = false;
                loop {
                    let packet_forwarder: OptionFuture<_> = if playing { Some(packet_forwarder_impl.clone()) } else { None }.into();
                    smol::pin!(packet_forwarder);
                    futures::select! {
                        pfr = packet_forwarder => {
                            //if o_video_playback_thread.is_some() { o_video_playback_thread.unwrap().flush_to_end().await }
                            eof = true;
                            sender.send(Media::EOF).await.unwrap_or(());
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
        self.control_sender.send_blocking(Self::CTL_DIE).unwrap_or(());
        self.control_sender.close();
        if let Some(decoder_thread) = self.demuxer_thread.take() {
            decoder_thread.thread();
        }
    }
}
