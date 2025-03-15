// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT




pub struct Player {
    window: x::Window,
    control_sender: smol::channel::Sender<(i64,String)>,
    demuxer_thread: Option<std::thread::JoinHandle<()>>,
    has_video: bool,
    has_audio: bool
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
    const CTL_VIDEO_DIE_SILENT:i64 = -128;
    const CTL_AUDIO_DIE_SILENT:i64 = -64;
    const MUX_DEAD:i64 = -32;
    const VIDEO_DEAD:i64 = -16;
    const AUDIO_DEAD:i64 = -8;
    const CTL_MUX_DIE:i64 = -4;
    const CTL_VIDEO_DIE:i64 = -2;
    const CTL_AUDIO_DIE:i64 = -1;

    const CTL_NEXT_AUDIO:i64 = 1;
    const CTL_SEEK_ABS_MIN:i64 = 1000+36000;
    const CTL_SEEK_ABS:i64 = 1000+36000*2;
    const CTL_SEEK_ABS_MAX:i64 = 1000+36000*3;
    const CTL_SEEK_REL_MIN:i64 = 1000+36000*4;
    const CTL_SEEK_REL:i64 = 1000+36000*5;
    const CTL_SEEK_REL_MAX:i64 = 1000+36000*6;
    const CTL_NONE:i64 = i64::MAX;
    
    pub fn send_ctl(&self,c:i64)->Result<(),smol::channel::SendError<(i64,String)>> {
        let blank = "".to_string();
        self.control_sender.send_blocking((c,blank.clone()))
    }
    
    pub fn send_ctl_ex(&self,c:i64,p:&str)->Result<(),smol::channel::SendError<(i64,String)>> {
           self.control_sender.send_blocking((c,p.to_string()))
    }
    pub fn check(path: PathBuf)->(i32,Option<Input>) {
        let mut ret: i32 = 0;
        let mut dic = ffmpeg_next::Dictionary::new();
        let o_input_context = ffmpeg_next::format::input_with_dictionary(&path,dic);
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
         //       println!("Slo {d}");
                if (end_pts-start_pts)/2 > (end_pts-pts_edge/2) {
                    d = 0;
                }
            }
        } 
        so = so + d;

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

    fn audio_stream(input_context: &Input,ofs:u32)->ffmpeg_next::format::stream::Stream {
        let vs = input_context.streams().best(Type::Video);
        let mut v = usize::MAX;

        if vs.is_some() {
            v = vs.unwrap().index();
        }
        let mut b = (input_context.streams().best(Type::Audio).unwrap().index() as u32 + ofs) % (input_context.streams().len() as u32);
       // println!("Finding audio: {b}");
        loop {
            for s in input_context.streams() {
       //         println!("Audio meta: {:?}",s.metadata());
                if s.index() as u32 == b {
                    if s.index() == v {
                        b = (b + 1) % (input_context.streams().len() as u32);
                        break;
                    } else { return s; }
                }
            }
            break;
        }
        input_context.streams().best(Type::Audio).unwrap()
    }

    fn video_stream(input_context: &Input)->ffmpeg_next::format::stream::Stream {
        for s in input_context.streams() {
            println!("Stream: {:?}",s);
        }
        input_context.streams().best(Type::Video).unwrap()
    }
    pub fn start(
        im: &Visual,
        drw: x::Drawable,
        drb: x::Drawable,
        file: &str,
        mut input_context: Input,
        settings: StreamSettings,
        sender: smol::channel::Sender<(i64,String)>,
        bb: x::Pixmap
    ) -> Result<Self, anyhow::Error> {
        let ctx = &CTX;
        ctx.hide(im.window);

        let (control_sender, control_receiver) = smol::channel::bounded::<(i64,String)>(512);
        let sen2 = sender.clone();
        let file2 = file.to_string();

        //let sen3 = sender.clone();
       // let con2 = control_receiver.clone();
        let fs = settings.frame_skip as i64;

        let m = im.clone();
        let mut has_audio = false;
        let mut has_video = false;
        if settings.use_audio && input_context.streams().best(ffmpeg_next::media::Type::Audio).is_some() { has_audio = true; }
        if settings.use_video && input_context.streams().best(ffmpeg_next::media::Type::Video).is_some() { has_video = true; }

        let load_audio = has_audio.clone();
        let mut load_video = has_video.clone();

        /*if !load_video {
            let mut dic = ffmpeg_next::Dictionary::new();
            dic.set("-filter_complex","ebur128=video=1[OUT]");
            dic.set("-map","[OUT]");
            dic.set("-r:v","30.000");
            dic.set("-c:v","libx264");
            dic.set("-pix_fmt:v","yuv420p");
            dic.set("-f","mp4");
            input_context = ffmpeg_next::format::input_with_dictionary(&file2,dic).unwrap();
            println!("Trying visuals");
            load_video = true;
        }*/

        let mut aso = 0;
        let demuxer_thread = std::thread::Builder::new().name("demuxer thread".into()).spawn(move || {
            smol::block_on(async move {
                let ctx = &CTX;
                sen2.send((Media::LOADED,"".to_string())).await.unwrap_or(());
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

                let o_video_playback_thread: Option<VideoPlaybackThread> = Option::None;
                let packet_forwarder_impl = async {
                    let mut d = i64::MIN;
                    let mut so: i64 = start_secs;
                    let mut chain_cmd = Self::CTL_NONE;
                    let mut chain_ex = "";

                    if load_audio && load_video {
                        loop {
                            let audio_stream = Self::audio_stream(&input_context, aso);
                            let audio_stream_index = audio_stream.index();
                            let mut video_stream = Self::video_stream(&input_context);
                            let video_stream_index = video_stream.index();

                            let audio_playback_thread_r = AudioPlaybackThread::start(false, &audio_stream, sender.clone());
                            if audio_playback_thread_r.is_err() {
                                aso += 1;
                                continue;
                            }
                            let audio_playback_thread = audio_playback_thread_r.unwrap();
                            let video_playback_thread = VideoPlaybackThread::start(&m, drw, drb, settings.clone(), &video_stream, sender.clone()).unwrap();
                            if d==-1 {
                                d =  i64::MIN;
                            }
                            loop {
                                if d != i64::MIN {
                                    if d == -1 {
                                        let mut dic = ffmpeg_next::Dictionary::new();
                                        input_context = ffmpeg_next::format::input_with_dictionary(&file2,dic).unwrap();
                                        //input_context = ffmpeg_next::format::input(&file2).unwrap();
                                        break;
                                    }
                                    if end_pts > d {
                                        start_pts = d;
                                        if input_context.seek(start_pts, ..end_pts).is_ok() {
                                            audio_playback_thread.send_control_message(chain_cmd).await;
                                            video_playback_thread.send_control_message_ex(chain_cmd, chain_ex).await;
                                        } else {
                                            println!("bad seek");
                                        }
                                    }
                                    d = i64::MIN;
                                }

                                for (stream, packet) in input_context.packets() {
                                    if stream.index() == audio_stream_index {
                                        audio_playback_thread.receive_packet(packet).await;
                                    } else if stream.index() == video_stream_index {
                                        while !control_receiver.is_empty() {
                                            let command = control_receiver.recv().fuse().await;
                                            //println!("AV command: {:?}",command);

                                            let cv = command.unwrap().0;
                                            match cv {
                                                Self::CTL_NEXT_AUDIO => {
                                                    aso += 1;
                                                    d = -1;
                                                }
                                                Self::CTL_MUX_DIE => {
                                                    return;
                                                }
                                                Self::CTL_AUDIO_DIE => {
                                                    audio_playback_thread.send_control_message(Self::CTL_AUDIO_DIE).await;
                                                }
                                                Self::CTL_VIDEO_DIE => {
                                                    video_playback_thread.send_control_message(Self::CTL_VIDEO_DIE).await;
                                                }
                                                _ => {
                                                    if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                        let npts = packet.pts().unwrap_or(0);
                                                        if cv > Self::CTL_SEEK_ABS {
                                                            so = Self::pts_delta(0, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        } else {
                                                            so = Self::pts_delta(end_secs, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        }
                                                        chain_cmd = Self::CTL_SEEK_ABS;
                                                        if d > npts {
                                                            chain_ex = "f";
                                                        } else {
                                                            chain_ex = "r";
                                                        }
                                                    } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                        let sd = cv - Self::CTL_SEEK_REL;

                                                        let npts = packet.pts().unwrap_or(0);
                                                        if npts != 0 {
                                                            start_pts = npts;
                                                            so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            chain_cmd = Self::CTL_SEEK_REL;
                                                            if sd > 0 {
                                                                chain_ex = "f";
                                                            } else {
                                                                chain_ex = "r";
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                 //       last_pts = packet.pts().unwrap_or(last_pts);
                                        if d != i64::MIN { break }
                                        video_playback_thread.receive_packet(packet).await;
                                    }
                                }
                                if d == i64::MIN {
                         //           println!("out of packets!");
                                    break
                                }
                            }
                            if d!=-1 {
                                audio_playback_thread.send_control_message(Self::CTL_AUDIO_DIE).await;
                                video_playback_thread.send_control_message(Self::CTL_VIDEO_DIE).await;
                            } else {
                                audio_playback_thread.send_control_message(Self::CTL_AUDIO_DIE_SILENT).await;
                                loop {
                                    futures::select! {
                                        _ = futures::FutureExt::fuse(smol::Timer::after(Duration::from_millis(10))) => {}
                                        cmd = control_receiver.recv().fuse() => {
                                            match cmd.unwrap().0 {
                                                Player::AUDIO_DEAD=> {
                                                    break;
                                                }
                                                _=>{}
                                            }
                                        }
                                    }
                                }
                                video_playback_thread.send_control_message(Self::CTL_VIDEO_DIE_SILENT).await;
                                loop {
                                    futures::select! {
                                        _ = futures::FutureExt::fuse(smol::Timer::after(Duration::from_millis(10))) => {}
                                        cmd = control_receiver.recv().fuse() => {
                                            match cmd.unwrap().0 {
                                                Player::VIDEO_DEAD=> {
                                                    break;
                                                }
                                                _=>{}
                                            }
                                        }
                                    }
                                }
                            }
                            if d == i64::MIN { break }
                       //     println!("Rethreading!");
                        }
                    } else if load_audio && !load_video && !m.visible {
                        let audio_stream = input_context.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
                        let audio_stream_index = audio_stream.index();
                        let audio_playback_thread = AudioPlaybackThread::start(true,&audio_stream,sender.clone()).unwrap();
                        loop {
                            if d != i64::MIN {
                                if end_pts > d {
                                    start_pts = d;
                                    if input_context.seek(start_pts, ..start_pts).is_ok() {
                                        audio_playback_thread.send_control_message(chain_cmd).await;
                                    }
                                }
                                d = i64::MIN;
                            }
                            for (stream, packet) in input_context.packets() {
                                if stream.index() == audio_stream_index {
                                    while !control_receiver.is_empty() {
                                        let command = control_receiver.recv().fuse().await;
                                        let cv = command.unwrap().0;
                                        match cv {
                                            Self::CTL_MUX_DIE => {
                                                return;
                                            }
                                            Self::CTL_AUDIO_DIE => {
                                                audio_playback_thread.send_control_message(cv).await;
                                            }
                                            _ => {
                                                    if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                        if cv > Self::CTL_SEEK_ABS {
                                                            so = Self::pts_delta(0, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        } else {
                                                            so = Self::pts_delta(end_secs,cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        }
                                                        chain_cmd = Self::CTL_SEEK_ABS;
                                                    } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                        let sd = cv - Self::CTL_SEEK_REL;

                                                        let npts = packet.pts().unwrap_or(0);
                                                        if npts != 0 {
                                                            start_pts = npts;
                                                            so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            chain_cmd = Self::CTL_SEEK_REL;
                                                        }
                                                    }
                                                }

                                        }
                                    }
                                    if d != i64::MIN { break }
                                    audio_playback_thread.receive_packet(packet).await;
                                }
                            }
                            if d == i64::MIN { break }
                        }
                        audio_playback_thread.send_control_message(Self::CTL_AUDIO_DIE).await;
                    } else if load_audio && !load_video && m.visible {
                        let audio_stream = input_context.streams().best(ffmpeg_next::media::Type::Audio).unwrap();
                        let audio_stream_index = audio_stream.index();
                        let audio_playback_thread = ViAudioPlaybackThread::start(&m, drw, drb, bb, true,&audio_stream,sender.clone()).unwrap();
                        loop {
                            if d != i64::MIN {
                                if end_pts > d {
                                    start_pts = d;
                                    if input_context.seek(start_pts, ..start_pts).is_ok() {
                                        audio_playback_thread.send_control_message(chain_cmd).await;
                                    }
                                }
                                d = i64::MIN;
                            }
                            for (stream, packet) in input_context.packets() {
                                if stream.index() == audio_stream_index {
                                    while !control_receiver.is_empty() {
                                        let command = control_receiver.recv().fuse().await;
                                        let cv = command.unwrap().0;
                                        match cv {
                                            Self::CTL_MUX_DIE => {
                                                return;
                                            }
                                            Self::CTL_AUDIO_DIE => {
                                                audio_playback_thread.send_control_message(cv).await;
                                            }
                                            _ => {
                                                if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                    if cv > Self::CTL_SEEK_ABS {
                                                        so = Self::pts_delta(0, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                        d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    } else {
                                                        so = Self::pts_delta(end_secs,cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                        d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                    }
                                                    chain_cmd = Self::CTL_SEEK_ABS;
                                                } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                    let sd = cv - Self::CTL_SEEK_REL;

                                                    let npts = packet.pts().unwrap_or(0);
                                                    if npts != 0 {
                                                        start_pts = npts;
                                                        so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                        d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        chain_cmd = Self::CTL_SEEK_REL;
                                                    }
                                                }
                                            }

                                        }
                                    }
                                    if d != i64::MIN { break }
                                    audio_playback_thread.receive_packet(packet).await;
                                }
                            }
                            if d == i64::MIN { break }
                        }
                        audio_playback_thread.send_control_message(Self::CTL_AUDIO_DIE).await;
                    } else if load_video && !load_audio {
                        let mut video_stream = input_context.streams().best(ffmpeg_next::media::Type::Video).unwrap();
                        let video_stream_index = video_stream.index();
                        let video_playback_thread = VideoPlaybackThread::start(&m, drw, drb, settings.clone(), &video_stream, sender.clone()).unwrap();
                        loop {
                            if d != i64::MIN {
                                if end_pts > d {
                                    start_pts = d;
                                    if input_context.seek(start_pts, ..start_pts).is_ok() {
                                        video_playback_thread.send_control_message_ex(chain_cmd, chain_ex).await;
                                    }
                                }
                                d = i64::MIN;
                            }
                            let pts_edge = rescale::Rescale::rescale(&30, (1, 1), rescale::TIME_BASE);
                            for (stream, packet) in input_context.packets() {
                                if stream.index() == video_stream_index {
                                    let ppts = packet.pts().unwrap();
                                    while !control_receiver.is_empty() && ppts > 0 {
                                        let command = control_receiver.recv().fuse().await;
                                        let cv = command.unwrap().0;
                                        match cv {
                                            Self::CTL_MUX_DIE => {
                                                return;
                                            }
                                            Self::CTL_VIDEO_DIE => {
                                                video_playback_thread.send_control_message(Self::CTL_VIDEO_DIE).await;
                                            }
                                            _ => {
                                                    if cv >= Self::CTL_SEEK_ABS_MIN && cv <= Self::CTL_SEEK_ABS_MAX {
                                                        let npts = packet.pts().unwrap_or(0);
                                                        if cv > Self::CTL_SEEK_ABS {
                                                            so = Self::pts_delta(0, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        } else {
                                                            so = Self::pts_delta(end_secs, cv - Self::CTL_SEEK_ABS, 0, end_pts);
                                                            d = rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                        }
                                                        chain_cmd = Self::CTL_SEEK_ABS;
                                                        if d > npts {
                                                            chain_ex = "r";
                                                        } else {
                                                            chain_ex = "r";
                                                        }
                                                    } else if cv >= Self::CTL_SEEK_REL_MIN && cv <= Self::CTL_SEEK_REL_MAX {
                                                        let sd = cv - Self::CTL_SEEK_REL;

                                                        let npts = packet.pts().unwrap_or(0);
                                                        if npts != 0 {
                                                            start_pts = npts;
                                                            so = Self::pts_delta(so, sd, start_pts, end_pts);
                                                            d = start_pts + rescale::Rescale::rescale(&so, (1, 1), rescale::TIME_BASE);
                                                            chain_cmd = Self::CTL_SEEK_REL;
                                                            if sd > 0 {
                                                                chain_ex = "f";
                                                            } else {
                                                                chain_ex = "r";
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                    }
                                    if d != i64::MIN { break }
                                    video_playback_thread.receive_packet(packet).await;
                                }
                            }
                            if d == i64::MIN { break }
                        }
                        video_playback_thread.send_control_message(Self::CTL_VIDEO_DIE).await;
                    }
                }.fuse().shared();

                let mut playing = true;
                let mut eof = false;
                ctx.show(m.window);
                loop {
                    let packet_forwarder: OptionFuture<_> = if playing { Some(packet_forwarder_impl.clone()) } else { None }.into();
                    smol::pin!(packet_forwarder);
                    futures::select! {
                        pfr = packet_forwarder => {
                //            println!("EOF!");
                            sen2.send((Media::EOF,"".to_string())).await;
                            break;
                        }
                    }
                }
                sen2.send((Player::MUX_DEAD,blank!())).await;
            });
        })?;

        Ok(Self {
            window: im.window,
            has_audio,
            has_video,
            control_sender,
            demuxer_thread: Some(demuxer_thread)
        })
    }

    pub fn kill(&mut self,events: &smol::channel::Receiver<(i64,String)>) {
       // println!("START KILL!");
        let ctx = &CTX;

        if self.has_audio {
            if self.send_ctl(Player::CTL_AUDIO_DIE).is_ok() {
       //         println!("AWAITING DEAD AUDIO");
                let mut idx = 25;
                loop {
                    if idx < 0 { break }
                    thread::sleep(Duration::from_millis(2));
                    let e = events.try_recv();
                    if e.is_ok() {
                        let er = e.unwrap();
                        if er.0 == Player::AUDIO_DEAD { break }
                    }
                    thread::sleep(Duration::from_millis(10));
                    idx -= 1;
                }
            }
        }
        if self.has_video {
            if self.send_ctl(Player::CTL_VIDEO_DIE).is_ok() {
   //             println!("AWAITING DEAD VIDEO");
                let mut idx = 25;
                loop {
                    if idx < 0 { break }
                    thread::sleep(Duration::from_millis(2));
                    let e = events.try_recv();
                    if e.is_ok() {
                        let er = e.unwrap();
                        if er.0 == Player::VIDEO_DEAD { break }
                    }
                    thread::sleep(Duration::from_millis(10));
                    idx -= 1;
                }
            }
        }


        if self.send_ctl(Player::CTL_MUX_DIE).is_ok() {
  //          println!("AWAITING DEAD MUX");
            let mut idx = 25;
            loop {
                if idx < 0 { break }
                thread::sleep(Duration::from_millis(2));
                let e = events.try_recv();
                if e.is_ok() {
                    let er = e.unwrap();
                    if er.0 == Player::MUX_DEAD { break }
                }
                thread::sleep(Duration::from_millis(10));
                idx -= 1;
            }
        }

    }
}

impl Drop for Player {
    fn drop(&mut self) {
     //   println!("DROPPING Player");

        if self.has_audio { self.send_ctl(Player::CTL_AUDIO_DIE).unwrap_or_default() }
        if self.has_video { self.send_ctl(Player::CTL_VIDEO_DIE).unwrap_or_default() }

        self.send_ctl(Player::CTL_MUX_DIE).unwrap_or_default();
        if let Some(decoder_thread) = self.demuxer_thread.take() {
            let t = decoder_thread.thread();
        }
        self.control_sender.close();
    }
}

