use std::fmt::Debug;
use std::fs;
use std::ops::Index;
use std::path::Path;
use std::str::FromStr;
use duct::cmd;
use lazy_static::lazy_static;

use rhai::serde::DynamicSerializer;
use serde_json::{to_string, Serializer};
use xcb::x::Pixmap;

struct Rhai {
    base: String,
    allow_nsfw: bool,
    scope: RScope<'static>,
    layers: laymap!()
}

#[derive(Clone,CustomType)]
struct DomApp {
    layers: domlays!(),
    window: Window,
    drw: Drawable,
    drb: Drawable,
    back_buffer: Pixmap,
    width: u16,
    height: u16,
    base: String
}

#[derive(Clone,CustomType)]
#[derive(Eq, Hash, PartialEq)]
struct DomLayer {
    name: String,
    file: String
}

#[derive(Clone, CustomType)]
pub struct DomElem  {
    layer: String,
    sel: String
}

lazy_static! {
    static ref CTX: Xcb = Xcb::new();
}

lazy_static! {
    static ref ENGINE: Engine = Rhai::build_engine();
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
struct Rescaler(ffmpeg_next::software::scaling::Context);
unsafe impl std::marker::Send for Rescaler {}

fn rgba_rescaler_for_frame(frame: &ffmpeg_next::util::frame::Video,ow:u32,oh:u32) -> Rescaler {
    Rescaler(
        ffmpeg_next::software::scaling::Context::get(
            frame.format(),
            frame.width(),
            frame.height(),
            Pixel::BGRA,
            ow,
            oh,
            ffmpeg_next::software::scaling::Flags::FAST_BILINEAR,
        ).unwrap(),
    )
}

struct Media {
    eof: bool,
    cur_file: String,
    player: Player,
    events: smol::channel::Receiver<(i64,String)>
}

#[derive(Clone)]
struct StreamSettings {
    use_audio: bool,
    use_video: bool,
    frame_skip: u16,
    speed_factor: f64,
    start_secs: f64,
    stop_secs: f64,
    start_p: f64,
    stop_p: f64,
    zoom: f32,
    scale_x: f32,
    scale_y: f32,
    auto_fit_weight: f32
}

impl Media {
    const EOF:i64 = -1;
    const ERR:i64 = -2;
    const LOADED:i64 = 1;
    
    pub fn detect_vol(file:&str)->f32 {
        let cmd = cmd!("ffmpeg","-i", file,"-ss","00:15:00","-t","00:10:00","-filter:a","ebur128","-map","0:a","-f","null","-").stderr_capture();
        let o = cmd.run().unwrap();
        let sr = String::from_utf8(o.stderr).unwrap();
        let si = sr.find("Summary:").unwrap();
        let s = &sr[si..];
     //   println!("Detect summary: {}",s);
        
        let li = s.find("LRA:").unwrap();
        let p1 = &s[li+5..];
        let le = p1.find("\n").unwrap();
        let lv = p1[0..le].to_owned().replace("LU","").to_owned().trim().to_string();
        println!("Detected volume: {}",lv);
        f32::from_str(&lv).unwrap_or(f32::NAN)
    }

    pub fn master_vol(flra:f32) {
        let mxp = 100;
        let mnp = 70;
        let mlra = 10.0;
        let adj = (mlra - flra)/mlra;
        let rng = mxp - mnp;

        let mv = format!("{}%",mnp + (rng as f32*adj) as i32);
        println!("Auto set volume: {}",mv);
        let cmd = cmd!("amixer","set","-D","pulse","Master",mv).stderr_capture().stdout_capture();
        let o = cmd.run().unwrap();
        //println!("Auto set reply: {} {}",String::from_utf8(o.stderr).unwrap(),String::from_utf8(o.stdout).unwrap());
    }

    pub fn new(app: &mut DomApp, to_die: &mut Vec<u32>, ffms: &mut HashMap<u32,Media>, all: &mut HashMap<String,Layer>, style: &mut Style, idx:u32, globalsr:&mut strmap!(), scope:&mut RScope, m: Visual, drw: x::Drawable, drb: x::Drawable, bb:x::Pixmap) -> Self {
        let ctx = &CTX;
        let settings = StreamSettings {
            use_audio: true,
            use_video: true,
            frame_skip: 0,
            speed_factor: 1.0,
            start_secs: 0.0,
            stop_secs: 0.0,
            start_p: 0.0,
            stop_p: 0.0,
            zoom: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            auto_fit_weight: 1.0
        };
        loop {
            //println!("Start media {idx}");
            let (sender, events) = smol::channel::unbounded();
            let mc = m.clone();
            let mut input = Option::None;
            let mut flags: i32 = 0;
            let mut evr = HashMap::new();
            let mut cur_file = "".to_string();

            let mut sett = settings.clone();
            for a in &mc.attrs {
                match a.0.as_str() {
                    "use-audio"=> { sett.use_audio = bool::from_str(a.1.as_str()).unwrap_or(true); }
                    "use-video"=> { sett.use_video = bool::from_str(a.1.as_str()).unwrap_or(true); }
                    "speed-factor"=> { sett.speed_factor = f64::from_str(a.1.as_str()).unwrap_or(1.0); }
                    "start-secs"=> { sett.start_secs = f64::from_str(a.1.as_str()).unwrap_or(0.0) }
                    _=> {}
                }
            }

            //if !globalsr.contains_key("playlist") { globalsr.insert("playlist".to_string(), "".to_string()); }
            //if !globalsr.contains_key("is_unwatched") { globalsr.insert("is_unwatched".to_string(), "false".to_string()); }
            //if !globalsr.contains_key("play_idx") { globalsr.insert("play_idx".to_string(),"-1".to_string()); }
            //globalsr.insert("play_idx".to_string(),"0".to_string());

            loop {
                let mut path = mc.content.clone();
                if &mc.content[0..1]=="?" {
                    evr = DomApp::eval(globalsr,scope,&mc.content);
                    path = evr["_"].clone();
                }
                let file = PathBuf::from(&path.clone());
                //println!("Checking {:?}",file);
                (flags,input) = Player::check(file.clone());

                if !sett.use_audio && (flags & Player::HAS_VIDEO) != 0 || sett.use_audio && (flags & Player::HAS_AUDIO) != 0 {
                    cur_file = path.clone();
                    println!("Playing: {}",cur_file);
                    if mc.attrs.contains_key("id") {
                        let ps = path.to_string().clone();
                        let pa = ps.split("/").clone();
                        let pv = pa.collect::<Vec<&str>>();
                        let mut si = 2;
                        let mut show = pv[pv.len() - si - 1].to_string().clone();
                        loop {
                            if si > pv.len() { break}
                            let c = pv[pv.len() - si].to_string().clone();
                            if c.len() > 5 && (&c[0..6] == "Season" || &c[0..6] == "Series") {
                                show = pv[pv.len() - si - 1].to_string();
                                break;
                            }
                            si+=1;
                        }
                        if globalsr.contains_key(&(mc.attrs["id"].clone() + "_show")) { globalsr.remove(&(mc.attrs["id"].clone() + "_show")); }
                        globalsr.insert(mc.attrs["id"].clone() + "_show", show.clone());
                        scope.set_value(mc.attrs["id"].clone() + "_show", show.clone());
                    }
                    break
                }
            }

            let iu = input.unwrap();
            //if (flags & Player::HAS_AUDIO)!=0 {
                //let dv = Self::detect_vol(cur_file.as_str());
                //if dv!=f32::NAN { Self::master_vol(dv); }
            //}
          //  println!("Audio: {:?}",iu.streams().best(ffmpeg_next::media::Type::Audio));

            let ply = Player::start(&m,drw,drb,cur_file.as_str(),iu,sett,sender,bb);
            if ply.is_ok() {
                let player = ply.ok().unwrap();

                for kv in evr {
                    let v = kv.1.clone();
                    let mut aa = kv.0.split(".");
                    let id = aa.nth(0).unwrap_or("?");
                    let act = aa.nth(0).unwrap_or("");
                    if id!="?" && v!="?" {
                        to_die.extend(app.apply_script_result(globalsr, scope, ffms, all, style, id, act, v));
                    }
                }

                return Media {
                    eof: false,
                    cur_file,
                    player,
                    events
                };
            } else {
                //println!("Playing failed");
            }
        }
    }
}

impl Drop for Media {
    fn drop(&mut self) {
        //println!("DROPPING Media");
    }
}

impl DomApp {
    const LONG_PRESS_DUR:f32 = 0.5;
    const DBL_CLICK_DUR:f32 = 0.25;
    
    fn new(base: String) ->Self {
        Self {
            layers: vec![],
            window: x::Window::none(),
            drw: x::Drawable::none(),
            drb: x::Drawable::none(),
            back_buffer: x::Pixmap::none(),
            width: 1,
            height: 1,
            base
        }
    }
    pub fn load_layer(&mut self,name: String,file: String) {
        self.layers.push((name.clone(),DomLayer::new(name,file)));
    }

    pub fn settings_all(file:String)->strmap!() {
        let path = bsf!(&file);
        let mut cnt = String::new();
        let mut f= File::open(&path);
        let mut vals: strmap!()=nmap!();
        if f.is_ok() {
            let mut fr = f.unwrap();
            fr.read_to_string(&mut cnt).unwrap();
            let mut lines = cnt.split("\n");
            loop {
                let lo = lines.nth(0);
                if lo.is_none() { break }

                let mut kv = lo.unwrap().split(":");
                let k = kv.next().unwrap();
                let v = kv.next();
                if v.is_some() {
                    vals.insert(k.to_string(), v.unwrap().to_string());
                }
            }
        }
        vals
    }

    pub fn settings_save(vals:strmap!(),file:String) {
        let path = bsf!(&file);
        fs::remove_file(path.clone()).unwrap_or(());
        let mut fo= File::options().write(true).create(true).open(path).unwrap();
        for vi in vals {
            let k = vi.0;
            let v = vi.1;
            fo.write_fmt(format_args!("{}:{}\n",k,v)).unwrap();
        }
        fo.flush().unwrap();
    }

    pub fn setting_get(name: String,file: String,default:String)->String {
        let all = Self::settings_all(file);
        if all.contains_key(name.as_str()) { all[&name].clone() }
        else { default.clone() }
    }
    pub fn setting_set(name: String,value: String,file:String) {
        let mut all = Self::settings_all(file.clone());
        if all.contains_key(&name) { all.remove(&name); }
        all.insert(name.to_string(),value.clone());

        Self::settings_save(all,file.clone());
    }

    fn eval(globals:&strmap!(),scope:&mut RScope, script: &str) ->HashMap<String,String> {
        let mut engine = &ENGINE;
        let mut eval = script.trim();
        if eval.starts_with("??=") { eval = &script[3..eval.len()-2]; }
        let mut torun = format!("let eve = result_init(curlib,base);");
        let mut torund = "".to_string();

        for g in globals {
            let id = "globals";
            let act = g.0;

            if act=="" { continue }
            //println!("Dumping global: {id}.{act}={:?}",g.1);

            let v= g.1;
            if &act[0..3] == "is_" {
                let mut nv = false;
                if v == "!" {
                    nv = !scope.get_value(act).unwrap_or(false);
                } else {
                    nv = bool::from_str(v.clone().as_str()).unwrap();
                }
                torun += &format!(r#"eve = result(eve,"{act}",{nv});"#);
                if scope.get_value::<bool>(act).is_none() ||  scope.get_value::<bool>(act).unwrap()!=nv {
                    scope.set_value(act,nv);
                    torund += &format!(r#"eve = on_event(eve,"{id}.{act}","change",0,0,0);"#);
                }
            } else if act == "play_idx" {
                let nv = i64::from_str_radix(v.clone().as_str(), 10).unwrap();
                torun += &format!(r#"eve = result(eve,"{act}",{nv});"#);
                if scope.get_value::<i64>(act).is_none() ||  scope.get_value::<i64>(act).unwrap()!=nv {
                    scope.set_value(act,nv);
                    torund += &format!(r#"eve = on_event(eve,"{id}.{act}","change",0,0,0);"#);
                }
            } else {
                let nv = v.clone().to_owned();
                torun += &format!(r#"eve = result(eve,"{act}","{nv}");"#);
                if scope.get_value::<String>(act).is_none() || scope.get_value::<String>(act).unwrap()!=nv {
                    scope.set_value(act,nv);
                    torund += &format!(r#"eve = on_event(eve,"{id}.{act}","change",0,0,0);"#);
                }
            }
        }
        if eval !="" {
            torund += &("eve = ".to_string() + eval + r#";return result_complete(eve);"#);
        } else {
            torund += r#"return result_complete(eve);"#
        }
        //println!("{torun}");
        let rs = engine.eval_with_scope::<String>(scope,&(script!("common","rhai").as_str().to_string() + ";" + torun.as_str() + torund.as_str())).unwrap().to_string();
        let mut res = rs.split('\n').collect::<Vec<&str>>();
        let mut ret = HashMap::new();
        let mut i = 0;
        while i<res.len()-1 {
            let k = res[i].to_string();
            let v = res[i+1].to_string();
            if k!="" && v!="?" {
             //   println!("Eval result: {k}={v}");
                ret.insert(k,v);
            }
            i+=2;
        }
        ret
    }

    fn main_loop(&mut self, iwidth:i64, iheight:i64) {
        let mut escope = RScope::new();
        let ctx = &CTX;
        let mut nwidth= iwidth as u16;
        let mut nheight = iheight as u16;
        self.window = ctx.master_window;
        ctx.size(self.window,nwidth,nheight);

        self.drw = Drawable::Window(self.window);
        let mut style = Style::new(self.drw, &ctx, "common");
        let mut ffms: HashMap<u32,Media> = nmap!();
        let mut all: laymap!() = nmap!();

        escope.push("curlib", "Music");
        escope.push("base", self.base.clone());
        let mut globals: strmap!() = nmap!();
        let globalsr = &mut globals;
        let mut res: Vec<(String,HashMap<String,String>)> = vec![];

        for dl in &self.layers {
            let mut nl = Layer::new(&dl.1.file, ctx, self.window,0,0);
            nl.visibility(true,ctx);
            all.insert(dl.1.name.clone(),nl);
            ctx.collect();
            res.push(("app-init".to_string(),DomApp::eval(globalsr,&mut escope,&format!(r#"on_event(eve,"{}","loaded",0,0,"")"#,dl.1.name))));
        }
        //res.clear();

        self.back_buffer = ctx.new_pixmap(self.drw, iwidth as u16, iheight as u16);
        self.drb = Drawable::Pixmap(self.back_buffer);

        let mut to_die: Vec<u32> = vec![];

        let mut win_up = false;
        let mut rebuild = false;

        let mut oha = Anim::new(&mut style,"@overlay","@overlay.passive",0);
        let mut cla = Anim::new(&mut style,"@mouse.down","@mouse.passive",0);
        let mut hoa = Anim::new(&mut style,"@mouse.hover","@mouse.exit",0);

        let mut motion_rid = 0;
        
        let mut bup_hist: HashMap<u8,(Instant,String)> = HashMap::new();
        let mut bdown_hist: HashMap<u8,(Instant,String)> = HashMap::new();
        let mut ignore_next: u8 = u8::MAX;
        
        
        loop {
            oha.update(self.window,&mut all,&mut style,Instant::now());
            hoa.update(self.window,&mut all,&mut style,Instant::now());
            cla.update(self.window,&mut all,&mut style,Instant::now());

            if to_die.contains(&0) {
                to_die.clear();
                rebuild = true;
            } else if !(nwidth!=self.width || nheight!=self.height || rebuild) {
                for f in ffms.iter_mut() {
                    let fr = f.1.events.try_recv();
                    if fr.is_ok() {
                     //   println!("Player ev: {:?}",fr);
                        let idx = *f.0;
                        let ev = fr.unwrap();
                        match ev.0 {
                            Player::VIDEO_DEAD=> {
                                f.1.player.send_ctl(ev.0).unwrap_or_default();
                            }
                            Player::AUDIO_DEAD=> {
                                f.1.player.send_ctl(ev.0).unwrap_or_default();
                            }
                            Media::ERR | Media::EOF => {
                                f.1.eof = true;
                                if f.1.player.has_audio { f.1.player.send_ctl(Player::CTL_AUDIO_DIE).unwrap_or_default() }
                                if f.1.player.has_video { f.1.player.send_ctl(Player::CTL_VIDEO_DIE).unwrap_or_default() }
                                f.1.player.send_ctl(Player::CTL_MUX_DIE).unwrap_or_default();
                                to_die.push(idx);
                            }
                            _ => {}
                        }
                    }
                }
            }

            if !rebuild && to_die.len() > 0 {
                let to_kill = to_die.clone();
                to_die.clear();
                for l in all.clone().iter() {
                    let mut medias = l.1.select("media");
                    let blank = "".to_string();
                    let m1 = "-1".to_string();
                    for m in medias.iter_mut() {
                        if globalsr.get("is_unwatched").unwrap_or(&blank) == "true" &&
                            m.attrs.get("id").unwrap_or(&blank) == "qfull" &&
                            globalsr.get("play_idx").unwrap_or(&blank) == globalsr.get("file_idx").unwrap_or(&m1) &&
                            globalsr.get("cur_show").unwrap_or(&blank) != "" {
                  //          println!("Play idx inc");
                            Self::setting_set("play_idx".to_string(), (i64::from_str_radix(&globalsr["play_idx"], 10).unwrap() + 1).to_string(), globalsr["cur_show"].to_string().clone());
                        }
                        let idx = m.window.resource_id();
                        if to_kill.contains(&idx) {
                            let mut f = ffms.remove(&idx).unwrap();
                            f.player.kill(&f.events);
                            drop(f);
                            let idx = m.window.resource_id().clone();
                    //        println!("New media {}x{}", m.width, m.height);
                            let med = Media::new(self, &mut to_die,&mut ffms, &mut all, &mut style, idx, globalsr, &mut escope, m.clone(), self.drw, self.drb,self.back_buffer);
                            ffms.insert(idx, med);
                        }
                    }
                }
                ctx.collect();
            } else {
                if rebuild { ctx.collect(); }
            }

            let gcb = ctx.new_gc(self.drw, 0, 0);
            //ctx.copy(gcb, self.drb, self.drw, 0, 0, 0, 0, self.width, self.height);
            for l in all.iter_mut() {
                let mut icons = l.1.select("media");
                icons.extend(l.1.select("i"));
                icons.extend(l.1.select("lbl"));

                for vi in icons {
                    if !vi.visible { continue }

                    let vwd = Drawable::Window(vi.window);
                    let vd = Drawable::Pixmap(vi.buf);
                    if vi.inv_mask != x::Pixmap::none() && vi.mask != x::Pixmap::none() {
                        if vi.bg!=0 {
                            let gc = ctx.new_gc(self.drw, vi.bg, vi.bg);
                            ctx.rect(gc, vwd, 0, 0, vi.width, vi.height);
                            ctx.drop_gc(gc);
                        } else {
                            let mgc_i = ctx.new_masked_gc(vwd, vi.inv_mask, vi.fg, vi.bg);
                            ctx.copy(mgc_i, self.drb, vwd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                            ctx.drop_gc(mgc_i);
                        }
                        let mgc = ctx.new_masked_gc(vwd, vi.mask, vi.fg, vi.bg);
                        ctx.copy(mgc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                        ctx.drop_gc(mgc);
                    } else if vi.buf != x::Pixmap::none() {
                        //ctx.copy(gcb, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                    }
                }
            }
            ctx.drop_gc(gcb);
        //    ctx.collect();

            let mut ev = XcbEvent::new();
            if !rebuild {
                let n = Instant::now();

                for h in bup_hist.clone().iter() {
                    if n - (*h.1).0 > Duration::from_secs_f32(Self::DBL_CLICK_DUR) {
                        let er = Self::eval(globalsr, &mut escope, &format!(r#"on_event(eve,"{}","b-up",0,0,{})"#, (*h.1).1, h.0));
                        res.push(("all".to_string(), er));
                        clear!(h.0,bup_hist);
                        clear!(h.0,bdown_hist);
                    }
                }
                
                for h in bdown_hist.clone().iter() {
                    if n - (*h.1).0 > Duration::from_secs_f32(Self::LONG_PRESS_DUR) {
                        if !bup_hist.contains_key(h.0) || bup_hist[h.0].0 < (*h.1).0 {
                            let er = Self::eval(globalsr,&mut escope, &format!(r#"on_event(eve,"{}","b-long",0,0,{})"#, (*h.1).1, h.0));
                            res.push(("all".to_string(),er));
                            clear!(h.0,bup_hist);
                            clear!(h.0,bdown_hist);
                            ignore_next = *h.0;
                        }
                    }
                }
                
                ev = ctx.wait_event()
            }
            
      //    println!("-{}-",ev.code);
            match ev.code {
                XcbEvent::CLOSE => {
                    std::process::exit(0);
                }
                XcbEvent::SCROLL_UP|XcbEvent::SCROLL_DOWN=> {
                    let rid = ev.window.resource_id();
                    for li in all.iter_mut() {
                        let l = li.1;
                        let vco = l.visual_by_res(rid);
                        if vco.is_some() {
                            let vc = vco.unwrap();
                            if vc.tag == "media" {
                                let mut dir = "up";
                                if ev.code == XcbEvent::SCROLL_DOWN { dir = "down" }

                                let er = Self::eval(globalsr, &mut escope, &format!(r#"on_event(eve,"{}","scroll-{}",0,0,{})"#, vc.attrs.get("id").unwrap_or(&"?".to_string()), dir, ev.button));
                                //println!(">> {} {:?}",li.0,er);
                                res.push((li.0.clone(), er));
                            }
                        }
                    }
                }
                XcbEvent::MOTION=>{
                    motion_rid = ev.window.resource_id();
                }
                XcbEvent::B_DOWN => {
                    if ev.button==1 {
                        let rid = ev.window.resource_id();
                        for li in all.iter_mut() {
                            let l = li.1;
                            let vco = l.visual_by_res(rid);
                            if vco.is_some() {
                                let vc = vco.unwrap();
                                if vc.attrs.contains_key("id") {
                                    ensure!(ev.button,(Instant::now(),vc.attrs["id"].clone()),bdown_hist);
                                }
                                if vc.tag != "choices" {
                                    cla.reset(&mut style, rid);
                                }
                            }
                        }
                    }
                }
                XcbEvent::B_UP => {
                    if ignore_next == ev.button {
                        ignore_next = u8::MAX;;
                    } else {
                        let rid = ev.window.resource_id();
                        for li in all.iter_mut() {
                            let l = li.1;
                            let vco = l.visual_by_res(rid);
                            if vco.is_some() {
                                let vc = vco.unwrap();
                                if vc.tag == "choices" {
                                    let mut cy = ev.y / Visual::DEF_LINE_H as i16;
                                    if ev.button != 1 || cy < 1 { break }
                                    cy -= 1;

                                    let cx = ev.x * Visual::DEF_CHOICE_COLS as i16 / vc.width as i16;
                                    let ca = vc.attrs.clone();
                                    let blank = "".to_string();
                                    let ss = ca.get("selected").unwrap_or(&blank);
                                    let n = (cx + cy * Visual::DEF_CHOICE_COLS as i16) as usize;
                                    let iss = ca.get("items").unwrap();
                                    let cco = iss.split(":").nth(n);
                                    //    println!("Choice check:\n {} => {:?} {cx}x{cy} [{n}]",iss,cco);
                                    if cco.is_some() {
                                        let cc = cco.unwrap();
                                        if ss != "" {
                                            let sp = ":".to_string() + cc + ":";
                                            let mut sh = ":".to_string() + ss.as_str() + ":";
                                            vc.attrs.remove("selected");
                                            if sh.contains(&sp) {
                                                sh = sh.replace(&sp, ":");
                                            } else {
                                                sh = sh + cc + ":";
                                            }
                                            if sh == ":" {
                                                vc.attrs.insert("selected".to_string(), "".to_string());
                                            } else {
                                                vc.attrs.insert("selected".to_string(), sh[1..sh.len() - 1].to_string());
                                            }
                                            //println!("New selected: {ss}:{cc}");
                                        } else {
                                            vc.attrs.remove("selected");
                                            vc.attrs.insert("selected".to_string(), cc.to_string());
                                            //println!("First selected: {cc}");
                                        }
                                        let v = Self::eval(globalsr, &mut escope, &vc.content.clone());
                                        vc.set_content(self.drw, ctx, &mut style, v["_"].as_str());
                                        //print!("RR");
                                        ctx.request_redraw(vc.window, 0, 0, vc.width, vc.height);
                                    }
                                } else {
                                    if vc.attrs.contains_key("id") {
                                        let id = vc.attrs["id"].clone();
                                        if bup_hist.contains_key(&ev.button) {
                                            let n = Instant::now();
                                            let l = bup_hist[&ev.button].0;

                                            if n - l < Duration::from_secs_f32(Self::DBL_CLICK_DUR) {
                                                let er = Self::eval(globalsr, &mut escope, &format!(r#"on_event(eve,"{}","b-dbl",0,0,{})"#, id, ev.button));
                                                res.push(("all".to_string(), er));
                                                clear!(ev.button,bup_hist);
                                                clear!(ev.button,bdown_hist);
                                            } else {
                                                ensure!(ev.button,(Instant::now(),id),bup_hist);
                                            }
                                        } else {
                                            ensure!(ev.button,(Instant::now(),id),bup_hist);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                XcbEvent::NONE => {
                    if nwidth!=self.width || nheight!=self.height || rebuild {
                        //println!("Rebuild");
                        rebuild = false;
                        self.width = nwidth;
                        self.height = nheight;
                        if self.back_buffer != x::Pixmap::none() {
                            ctx.drop_pixmap(self.back_buffer);
                        }
                        self.back_buffer = ctx.new_pixmap(self.drw, self.width, self.height);
                        self.drb = Drawable::Pixmap(self.back_buffer);

                        for l in all.iter_mut() {
                            l.1.fit_all(self.drw, ctx, &mut style, self.width, self.height);
                        }

                        for l in all.iter_mut() {
                            let lc = l.1.clone();
                            let mut on_loaders = lc.select("choices");
                            for m in on_loaders {
                                let sc = Self::eval(globalsr,&mut escope, &m.content.clone());
                                let uv = l.1.visual_by_res(m.window.resource_id()).unwrap();
                                uv.set_content(self.drw, ctx, &mut style, &sc["_"].clone());
                                if m.attrs.contains_key("id") {
                                    escope.set_value(m.attrs["id"].to_owned() + "_selected","");
                                }
                            }
                        }

                        let mut l = all.get_mut("players").cloned().unwrap();
                        let mut medias = l.select("media");
                        let ka = ffms.keys().clone().map(|v|v.to_owned()).collect::<Vec<u32>>().clone();
                        for rid in ka.iter() {
                            let mut f = ffms.remove(rid).unwrap();
                            f.player.kill(&f.events);
                            drop(f);
                        }
                        for m in medias {
                            let idx = m.window.resource_id().clone();
                            let med = Media::new(self, &mut to_die,&mut ffms, &mut all, &mut style, idx, globalsr, &mut escope, m.clone(), self.drw, self.drb,self.back_buffer);
                            ffms.insert(idx, med);
                        }
                    }

                    for ri in &res {
                        let er = &ri.1;
                        for kv in er {
                            if kv.0=="_" && kv.1=="?" { continue }

                            let v = kv.1.clone();
                            let mut aa = kv.0.split(".");

                            let id = aa.nth(0).unwrap_or("");
                            let act = aa.nth(0).unwrap_or("");
                            //    println!("Apply: {}",kv.0);
                            let mr = self.apply_script_result(globalsr,&mut escope,&mut ffms,&mut all,&mut style,id,act,v);
                            to_die.extend(&mr);
                        }
                    }
                    res.clear();


                    let rid = motion_rid;
                    if rid!=0 {
                        oha.reset(&mut style, rid);
                        if rid != hoa.target {
                            hoa.apply_end(self.window, &mut all, &mut style);
                            for li in all.iter_mut() {
                                let l = li.1;
                                let vco = l.visual_by_res(rid);
                                if vco.is_some() {
                                    let vc = vco.unwrap();
                                    if vc.tag == "i" || vc.tag == "lbl" {
                                        hoa.reset(&mut style, rid);
                                        break;
                                    }
                                }
                            }
                        } else {
                            hoa.reset(&mut style, rid);
                        }
                        motion_rid = 0;
                    }

                    ctx.collect();
                    thread::sleep(std::time::Duration::from_millis(10));

                    if !win_up {
                        ctx.show(self.window);
                        win_up = true;
                    }
                }
                XcbEvent::RESIZE => {
                    if self.width != ev.width || self.height != ev.height {
                        nwidth = ev.width;
                        nheight = ev.height;
                    }
                }
                XcbEvent::RENDER => {
                    for l in all.iter_mut() {
                        let vo= l.1.visual_by_res(ev.window.resource_id());
                        if vo.is_some() {
                            let vi = vo.unwrap();
                            if vi.visible && vi.tag=="choices" {
                              //  print!("@{}@",&vi.attrs["id"]);
                                let vd = Drawable::Pixmap(vi.buf);
                                let vwd = Drawable::Window(vi.window);
                                let gc = ctx.new_gc(vd, vi.fg, vi.bg);
                                ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                                ctx.drop_gc(gc);
                            }
                        } //else {
                     //       print!("?");
                       // }
                    }
                }
                _ => {}
            }

        }
    }

    fn apply_script_result(&mut self,globalsr: &mut strmap!(),escope: &mut RScope,ffms:&mut HashMap<u32,Media>,all: &mut HashMap<String, Layer>,style: &mut Style, id: &str, act: &str, mut v: String)->Vec<u32> {
        let mut ret = vec![];
        if id=="_" && v=="?" { return ret; }
        let mut more: Vec<HashMap<String,String>> = vec![];

       // println!("Appying result: {id}->{act}={v}");
        let ctx = &CTX;

        match id {
            "app"=> {
                if act=="control" && v=="quit" {
                    std::process::exit(0);
                }
            }
            "globals"=> {
                if !globalsr.contains_key(act) || globalsr.get(act).unwrap().as_str() != v.as_str() {
                    if globalsr.contains_key(act) { globalsr.remove(act); }
                    //println!("Caching global: {act}={v}");
                    globalsr.insert(act.to_string(), v.clone());
                    more.push(DomApp::eval(globalsr, escope, ""));
                }
            }
            "layers"=> {
                let ka = ffms.keys().clone().map(|v|v.to_owned()).collect::<Vec<u32>>().clone();
                for rid in ka.iter() {
                    let mut f = ffms.remove(rid).unwrap();
                    //println!("DROP MEDIA");
                    f.player.kill(&f.events);
                    drop(f);
                }

                let mut files :HashMap<String,String> = nmap!();
                let ka = all.keys().map(|s|s.clone()).collect::<Vec<String>>();;
                for k in ka {
              //      let k = all.keys().nth(0).unwrap().clone();
                    //println!("Wiping layer: {:?}",k);
                    let l = all.remove(&k).unwrap();
                    files.insert(k,l.file.clone());

                    /*let mut medias = l.select("media");
                    for m in medias.iter_mut() {
                        let idx = m.window.resource_id();
                        if ffms.contains_key(&idx) {
                            ret.push(idx);
                        }
                    }*/
                    l.root_visual.demolish(ctx);
                    drop(l);

                    //if all.len() == 0 { break }
                }



                for ln in &self.layers {
                    //println!("Building layer: {:?}",ln.0);
                    let mut file = files.get(ln.0.as_str()).unwrap().clone();
                    if ln.0 == act {
                        file = v.clone() + ".view";
                    }
                    let mut nl = Layer::new(file.as_str(), ctx, self.window,0,0);
                    nl.fit_all(self.drw,ctx,style,self.width,self.height);
                    //if ln.1.name!="players" { nl.visibility(true,ctx); }
                    nl.visibility(true,ctx);
                    all.insert(ln.0.to_string(),nl);

                    more.push(DomApp::eval(globalsr,escope,&format!(r#"on_event(eve,"{}","loaded",0,0,"")"#,ln.0)));
                }
                ctx.collect();
           //     println!("Layers changed");
                //ret = true;
                ret.push(0);
            }
            _ => {
                match act {
                    "clone" => {
                        let l = all.get_mut("players").unwrap();
                        let vso = l.select_visual(&("#".to_string() + v.as_str()));
                        if vso.is_none() { return ret }

                        let vs = vso.unwrap().clone();
                        let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                        let rid = vr.window.resource_id();

                        let fs = ffms.get(&vs.window.resource_id()).unwrap().cur_file.clone();
                        let mut f = ffms.remove(&rid).unwrap();
                        f.player.kill(&f.events);
                        drop(f);
                        let mut mv = vr.clone();
                        mv.content = fs;
                        let nf = Media::new(self,&mut ret,ffms,all,style,rid,globalsr,escope,mv,self.drw, self.drb,self.back_buffer);
                        ffms.insert(rid, nf);
                    }
                    _ => {}
                }
                match v.as_str() {
                    "die"=> {
                  //      println!("Die ID: {id}");
                        if id=="*" {
                            //let ka = ffms.keys().clone().map(|v|v.to_owned()).collect::<Vec<u32>>().clone();
                            //for rid in ka.iter() {
                                //let mut f = ffms.remove(rid).unwrap();
                                //println!("Push kill");
                              //  ret.push(*rid);
                                //f.player.kill(&f.events);
                                //drop(f);
                            //}
                            ret.push(0);
                //            let ka = ffms.keys().clone().map(|v|v.to_owned()).collect::<Vec<u32>>().clone();
                  //          for rid in ka.iter() {
                                //let mut f = ffms.get_mut(&rid).unwrap();
                                //println!("TRIGGERING MEDIA EOF");
                                //f.player.kill(&f.events);
                                //drop(f);
                      //          ret.push(rid.clone());
                    //        }
                        } else {
                            let vis = all.get_mut("players").unwrap().select_visual(&("#".to_string()+id));
                            if vis.is_some() {
                                let vis = vis.unwrap().clone();
                                let rid = vis.window.resource_id();
                                //if ffms.contains_key(&rid) {
                                    //println!("Push die: {id}");
                                    ret.push(rid);
                                //}
                            } else {
                                println!("No visual ID");
                            }
                        }
                    }
                    _ => {}
                }
                for li in all.iter_mut() {
                    let l = li.1;
                    let lk = li.0;
                    if l.select_visual(&("#".to_string() + id)).is_none() { continue }

                    match act {
                        "control"=> {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let s = format!("{}.{}",vr.tag,v);
                            match s.as_str() {
                                "media.next-audio"=> {
                                    ffms.get_mut(&vr.window.resource_id()).unwrap().player.send_ctl(Player::CTL_NEXT_AUDIO).unwrap_or_default();
                                }
                                _=>{}
                            }
                        }
                        "checked" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let mut nv = !vr.checked;
                            if v != "!" {
                                nv = bool::from_str(&v).unwrap();
                            }
                            if nv!=vr.checked {
                                vr.checked = nv;
                                style.apply(vr, vr.pwidth, vr.pheight);
                                vr.make_assets(Drawable::Window(ctx.master_window), ctx, style);
                            }
                        }
                        "visible" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let mut nv = !vr.visible;
                            if v != "!" {
                                nv = bool::from_str(&v).unwrap();
                            }
                            if nv!=vr.visible {
                                vr.visible = nv;
                                if vr.visible { vr.show(ctx) } else {
                                    vr.hide(ctx);
                                    if vr.tag == "choices" && vr.attrs.contains_key("id") {
                                        let id = &vr.attrs["id"].to_owned();
                                        let blank = "".to_string();
                                        let lsel = vr.attrs.get("__last-selected").unwrap_or(&blank).clone();
                                        let sel = vr.attrs.get("selected").unwrap_or(&blank).clone();
                                        if lsel != sel {
                                            if globalsr.contains_key(&(id.to_owned() + "_selected")) { globalsr.remove(&(id.to_owned() + "_selected")); }
                                            globalsr.insert(id.to_owned() + "_selected", sel.clone());
                                            more.push(DomApp::eval(globalsr, escope, &format!(r#"on_event(eve,"{id}.selected","change",0,0,"{sel}")"#)));
                                            if vr.attrs.contains_key("__last-selected") { vr.attrs.remove("__last-selected"); }
                                            vr.attrs.insert("__last-selected".to_string(), sel.clone());
                                        }
                                    }
                                }
                            }
                        }
                        "content" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            if vr.content!=v {
                                vr.set_content(self.drw, ctx, style, &v);
                            }
                        }
                        "seek-rel" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let d: i64 = i64::from_str(&v).unwrap();
                            let f = ffms.get_mut(&vr.window.resource_id()).unwrap();
                            let _ = f.player.send_ctl(Player::CTL_SEEK_REL + d).unwrap_or(());
                        }
                        "seek-abs" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let d: i64 = i64::from_str(&v).unwrap();
                            let f = ffms.get_mut(&vr.window.resource_id()).unwrap();
                            let _ = f.player.send_ctl(Player::CTL_SEEK_ABS + d).unwrap_or(());
                        }
                        _ => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            if &act[0..3]=="is_" {
                                if v=="!" && vr.attrs.contains_key(act) {
                                    v = (!bool::from_str(&vr.attrs[act]).unwrap()).to_string();
                                } else {
                                    v = true.to_string();
                                }
                            }
                            if vr.attrs.contains_key(act) { vr.attrs.remove(act); }
                            vr.attrs.insert(act.to_string(), v.to_string());
                            vr.make_assets(Drawable::Window(ctx.master_window), &ctx, style);
                            more.push(DomApp::eval(globalsr,escope,&format!(r#"on_event(eve,"{id}.{act}","change",0,0,"{v}")"#)));
                        }
                    }
                }
            }
        }

        if more.len()>0 {
            for mr in more {
                for ei in mr {
                    let mut ia = ei.0.split(".");
                    let id = ia.nth(0).unwrap_or("").to_string();
                    let act = ia.nth(0).unwrap_or("").to_string();
                    ret.extend(self.apply_script_result(globalsr,escope, ffms, all, style, &id, &act, ei.1.clone()));
                }
            }
        }
        //println!("ASR: {id}.{act}={v} \n {:?}",ret);
        ret
    }
}


impl DomLayer {
    fn new(name:String,file: String)->Self {
        Self {
            name,
            file
        }
    }
  /*  pub fn query(self, sel:&str) ->DomElem {
        DomElem::new(self.name,sel)
    }*/
}
/*
impl DomElem {
    pub fn new(layer:String,sel:&str) ->Self {
        Self {
            layer,
            sel: sel.to_string()
        }
    }
    fn set_content(&mut self,value: String) {
        println!("Content ::: {value}");
    }
    fn get_content(&mut self)->String {
        "".to_string()
    }
}*/

impl Rhai {
    pub fn build_engine()->Engine {
        let mut e = Engine::new();
        e.set_max_expr_depths(1000,1000);
        e.set_max_functions(1000);

        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut e);
        e.register_global_module(RandomPackage::new().as_shared_module());
        //e.register_fn("replace_layer",DomApp::replace_layer);
        e.register_fn("setting_get",DomApp::setting_get);
        e.register_fn("setting_set",DomApp::setting_set);
        e.register_type::<DomApp>().register_fn ("new_app",DomApp::new)
            .register_fn("load_layer",DomApp::load_layer)
            .register_fn("main_loop",DomApp::main_loop);
        e
    }
    pub fn run(&mut self) {
        self.exec(&format!("startup({},\"{}\",false);",self.allow_nsfw,self.base));
    }
    pub fn exec(&mut self, mut script:&str) {
        let engine = &ENGINE;
        let mut eval = script!("common","rhai");
        eval += "\n";
        eval += script.trim();
        engine.run_with_scope(&mut self.scope,&eval).unwrap();
    }

    fn new(args: Vec<String>)-> Self {
        let mut scope = RScope::new();
        let base = args[1].clone();
        let allow_nsfw = args.contains(&"-nsfw".to_string());
        Self {
            base,
            allow_nsfw,
            scope,
            layers: nmap!(),
        }
    }
}