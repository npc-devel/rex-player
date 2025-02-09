// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT


use std::path::PathBuf;
use ffmpeg_next::rescale;
use futures::TryFutureExt;
use smol::stream::StreamExt;

#[derive(Clone, Copy)]
pub enum ControlCommand {
    Play,
    Pause,
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
        mut input_context: Input,
        settings: StreamSettings,
        video_frame_callback: impl FnMut(&ffmpeg_next::util::frame::Video) + Send + 'static,
        sender: smol::channel::Sender<i32>
    ) -> Result<Self, anyhow::Error> {
        let (control_sender, control_receiver) = smol::channel::unbounded();
        let sen2 = sender.clone();

        let demuxer_thread =
          std::thread::Builder::new().name("demuxer thread".into()).spawn(move || {
              smol::block_on(async move {
                  //let mut input_context = ffmpeg_next::format::input(&path).unwrap();
                  sender.send(Media::LOADED).await.unwrap_or(());
                  let mut start_pts = 0;
                  if settings.start_secs != 0.0 {
                      let start_secs: i64 = settings.start_secs as i64;
                      let d = rescale::Rescale::rescale(&start_secs,(1,1), rescale::TIME_BASE);
                      let dur = input_context.duration();

                      if settings.start_secs < 0.0 {
                          if d.abs() < dur { start_pts = dur + d }
                      } else {
                          start_pts = d;
                      }
                      input_context.seek(start_pts, ..start_pts).unwrap();
                  }

                  let video_stream =
                      input_context.streams().best(ffmpeg_next::media::Type::Video).unwrap();
                  let video_stream_index = video_stream.index();

                  let video_playback_thread = VideoPlaybackThread::start(
                      start_pts,
                      settings.clone(),
                      &video_stream,
                      Box::new(video_frame_callback),
                      sender
                  ).unwrap();

                  let mut playing = true;
                  if settings.use_audio && input_context.streams().best(ffmpeg_next::media::Type::Audio).is_some() {
                      let audio_stream =
                          input_context.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
                      let audio_stream_index = audio_stream.index();
                      let audio_playback_thread =
                          AudioPlaybackThread::start(&audio_stream).unwrap();

                      let packet_forwarder_impl = async {
                          let mut pts: i64 = -1;
                          for (stream, packet) in input_context.packets() {
                              pts = packet.pts().unwrap_or(0);
                              if stream.index() == audio_stream_index {
                                  audio_playback_thread.receive_packet(packet).await;
                              } else if stream.index() == video_stream_index {
                                  video_playback_thread.receive_packet(packet).await;
                              }
                          }
                          pts
                      }.fuse().shared();

                      let mut lpfr: i64 = 0;
                      loop {
                          let packet_forwarder: OptionFuture<_> =
                              if playing { Some(packet_forwarder_impl.clone()) } else { None }.into();

                          smol::pin!(packet_forwarder);

                          futures::select! {
                              pfr = packet_forwarder => {
                                      let tpfr:i64 = pfr.unwrap_or(i64::MIN);
                                      if lpfr == tpfr {
                                          sen2.send(Media::EOF).await.unwrap();
                                          break;
                                      }
                                      lpfr = tpfr;
                                  }, // playback finished
                              received_command = control_receiver.recv().fuse() => {
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
                                              }
                                          }
                                      }
                                      Err(_) => {
                                          sen2.send(Media::EOF).await.unwrap();
                                          return;
                                      }
                                  }
                              }
                          }
                      }
                  } else {
                      let packet_forwarder_impl = async {
                          let mut pts: i64 = -1;
                          for (stream, packet) in input_context.packets() {
                              pts = packet.pts().unwrap_or(0);
                              if stream.index() == video_stream_index {
                                  video_playback_thread.receive_packet(packet).await;
                              }
                          }
                          pts
                      }.fuse().shared();

                      let mut lpfr: i64 = 0;
                      loop {
                          let packet_forwarder: OptionFuture<_> =
                              if playing { Some(packet_forwarder_impl.clone()) } else { None }.into();

                          smol::pin!(packet_forwarder);
                          futures::select! {
                              pfr = packet_forwarder => {
                                      let tpfr:i64 = pfr.unwrap_or(i64::MIN);
                                      //println!("{tpfr}");
                                      if lpfr == tpfr {
                                          sen2.send(Media::EOF).await.unwrap();
                                          break;
                                      }
                                      lpfr = tpfr;
                                  },
                              received_command = control_receiver.recv().fuse() => {
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
                                              }
                                          }
                                      }
                                      Err(_) => {
                                          sen2.send(Media::EOF).await.unwrap();
                                          return;
                                      }
                                  }
                              }
                          }
                      }
                  }
                  sen2.send(Media::EOF).await.unwrap();
              });
              //sen2.send_blocking(Media::EOF).unwrap();
          })?;

        Ok(Self {
            control_sender,
            demuxer_thread: Some(demuxer_thread)
        })
    }

    /*pub fn toggle_pause_playing(&mut self) {
        if self.playing {
            self.playing = false;
            self.control_sender.send_blocking(ControlCommand::Pause).unwrap();
        } else {
            self.playing = true;
            self.control_sender.send_blocking(ControlCommand::Play).unwrap();
        }
        (self.playing_changed_callback)(self.playing);
    }*/
}

impl Drop for Player {
    fn drop(&mut self) {
        self.control_sender.close();
        if let Some(decoder_thread) = self.demuxer_thread.take() {
            decoder_thread.join().unwrap();
        }
    }
}
