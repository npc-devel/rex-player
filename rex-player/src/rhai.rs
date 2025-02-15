use std::path::Path;
use std::str::FromStr;
use lazy_static::lazy_static;

use rhai::serde::DynamicSerializer;
use serde_json::Serializer;

struct Rhai {
    /*ctx: Xcb,
    pub window: x::Window,
    back_buffer: x::Pixmap,
    width:u16,
    height:u16,*/
    engine: Engine,
    scope: RScope<'static>,
    //style: Style,
    layers: laymap!()
}

#[derive(Clone,CustomType)]
struct DomApp {
    layers: domlays!()
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
    cur_file: String,
    player: Player,
    events: smol::channel::Receiver<i32>
}

#[derive(Clone)]
struct StreamSettings {
    use_audio: bool,
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
    const EOF:i32 = -1;
    const ERR:i32 = -2;
    const LOADED:i32 = 1;

    pub fn new(idx:u32,scope:&mut RScope,m: Visual, drw: x::Drawable, drb: x::Drawable) -> Self {
        let ctx = &CTX;
        let settings = StreamSettings {
            use_audio: true,
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
            println!("Start media {idx}");
            let (sender, events) = smol::channel::unbounded();
            let mc = m.clone();
            let mut input = Option::None;
            let mut flags: i32 = 0;
            let mut evr = HashMap::new();
            let mut cur_file = "".to_string();

            let mut sett = settings.clone();
            for a in mc.attrs {
                match a.0.as_str() {
                    "use-audio"=> { sett.use_audio = bool::from_str(a.1.as_str()).unwrap_or(true); }
                    "speed-factor"=> { sett.speed_factor = f64::from_str(a.1.as_str()).unwrap_or(1.0); }
                    "start-secs"=> { sett.start_secs = f64::from_str(a.1.as_str()).unwrap_or(0.0) }
                    _=> {}
                }
            }

            loop {
                let mut path = mc.content.clone();
                if &mc.content[0..1]=="?" {
                    evr = DomApp::eval(scope,&mc.content);
                    path = evr["_"].clone();
                }
                let file = PathBuf::from(&path);
                println!("Checking {:?}",file);
                (flags,input) = Player::check(file.clone());

                if !sett.use_audio && (flags & Player::HAS_VIDEO) != 0 || sett.use_audio && (flags & Player::HAS_AUDIO) != 0 {
                    println!("Accepting {:?}",file);
                    cur_file = path;
                    break
                }
            }

            let ply = Player::start(&m,drw,drb,input.unwrap(), sett, sender);
            if ply.is_ok() {
                let player = ply.ok().unwrap();
                return Media {
                    cur_file,
                    player,
                    events
                };
            } else {
                println!("Playing failed");
            }
        }
    }
}

impl DomApp {
    fn new() ->Self {

        Self {
            layers: vec![]
        }
    }
    pub fn load_layer(&mut self,name: String,file: String) {
        self.layers.push((name.clone(),DomLayer::new(name,file)));
    }

    pub fn replace_layer(&mut self,name: String,file: String) {
        /*let ol = self.layers.remove(&name);
        drop(ol);
        self.layers.insert(name.clone(),DomLayer::new(name,file));*/
    }

    fn eval(scope:&mut RScope, script: &str) ->HashMap<String,String> {

        let mut engine = Engine::new();
        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut engine);
        engine.register_global_module(RandomPackage::new().as_shared_module());
        engine.register_fn("replace_layer",Self::replace_layer);


        let mut eval = script.trim();
        if eval.starts_with("??=") {
            let l = eval.len();
            eval = &script[3..l-2];

        }
        let eval = r#"let eve = result_init();eve = "#.to_string() + eval + r#";return result_complete(eve);"#;

        let rs = engine.eval_with_scope::<String>(scope,&(script!("common","rhai").as_str().to_string() + ";" + eval.as_str())).unwrap().trim().to_string();
        println!("\n{rs}\n");
        let mut res = rs.split('\n').collect::<Vec<&str>>();
        let mut ret = HashMap::new();
        while res.len()>0 {
            let v = res.pop().unwrap_or("").to_string();
            let k = res.pop().unwrap().to_string();
            ret.insert(k,v);
        }
        println!("{:?}", ret);
        ret
    }

    fn main_loop(&mut self, iwidth:i64, iheight:i64) {
        let mut escope = RScope::new();
        let ctx = &CTX;
        let mut nwidth= iwidth as u16;
        let mut nheight = iheight as u16;
        let mut width= 2;
        let mut height = 2;

        let window = ctx.new_window(0xFFFF1010);
        ctx.size(window,nwidth,nheight);

        let drw = Drawable::Window(window);
        let style = Style::new(drw,&ctx, "common");
        let mut ffms: HashMap<u32,Media> = nmap!();
        let mut all: laymap!() = nmap!();
        for dl in &self.layers {
            let mut nl = Layer::new(&dl.1.file, ctx, window,0,0);
            nl.visibility(true,ctx);
            all.insert(dl.1.name.clone(),nl);
            ctx.collect();
        }
        ctx.show(window);

        let mut back_buffer = ctx.new_pixmap(drw, iwidth as u16, iheight as u16);
        let mut drb = Drawable::Pixmap(back_buffer);

        let mut to_die: Vec<u32> = vec![];
        escope.push("curlib", "Videos/TV");
        //let mut swap_layers: strmap!() = nmap!();
        let mut rebuild = false;
        loop {

           /* for s in swap_layers.iter_mut() {
                println!("Swap layers {:?}", s);
                if s.0 == "players" {
                    let l = all.get("players").unwrap();
                    let mut medias = l.select("media");
                    for m in medias.iter_mut() {
                        let idx = m.window.resource_id();
                        let f = ffms.remove(&idx).unwrap();
                        drop(f);
                    }
                }

                let mut ol = all.clone();
                all.clear();
                for l in ol {
                    if l.0.as_str() == s.0.as_str() {
                        l.1.visibility(false,ctx);
                        //let o = ol.remove(&s.0.to_string()).unwrap();
                        drop(l.1);
                        let mut nl = Layer::new(&(s.1.to_string() + ".view"), ctx, window,0,0,width,height);
                        //nl.visibility(true,ctx);
                        all.insert(s.0.to_string(),nl);
                        rebuild = true;
                    } else {
                        all.insert(l.0,l.1);
                    }
                }

                /*if s.0 == "players" {
                    let l = all.get("players").unwrap();
                    let mut medias = l.select("media");
                    for m in medias.iter_mut() {
                        let idx = m.window.resource_id();
                        let nf = Media::new(idx,&mut escope, m.clone(), drw, drb);
                        ffms.insert(idx, nf);
                    }
                }*/
            }
            swap_layers.clear();*/
            /*if rebuild {
                for li in &self.layers {
                    let l = all.get_mut(&li.0).unwrap();
                    l.visibility(true,ctx);
                    l.fit_all(drw,ctx,&style,width,height);

                    ctx.collect();
                }
            }*/

            //println!("Loop");
            for f in ffms.iter() {
                let fr = f.1.events.try_recv();
                if fr.is_ok() {
                    let idx = *f.0;
                    let ev = fr.unwrap();
                    match ev {
                        Media::ERR|Media::EOF => {
                            println!("Killing {idx}");
                            to_die.push(idx);
                        }
                        _ => {}
                    }
                }
            }

            //println!("Die");
            if to_die.len()>0 {
                let l = all.get("players").unwrap();
                let mut medias = l.select("media");
                for m in medias.iter_mut() {
                    let idx = m.window.resource_id();
                    if to_die.contains(&idx) || rebuild {
                        let f = ffms.remove(&idx).unwrap();
                        drop(f);
                        let nf = Media::new(idx,&mut escope, m.clone(), drw, drb);
                        ffms.insert(idx, nf);
                    }
                }
                to_die.clear();
            }


            //println!("Fetch ev");
            let ev = ctx.wait_event();
            //println!("{:?}",ev);
            match ev.code {
                XcbEvent::B_DOWN => {
                    let rid = ev.window.resource_id();
                    let mut res: Vec<(String,HashMap<String,String>)> = vec![];
                    for li in all.iter_mut() {
                        let l = li.1;
                        let vco = l.visual_by_res(rid);
                        if vco.is_some() {
                            let vc = vco.unwrap();
                            res.push((li.0.clone(),DomApp::eval(&mut escope, &format!(r#"on_event(eve,"{}","b_down",0,0,{})"#, vc.attrs.get("id").unwrap_or(&"?".to_string()), ev.button))));
                        }
                    }
                    for ri in res {
                        let er = ri.1;
                        println!("EVAL RES {:?}", er);
                        for kv in er {
                            let mut aa = kv.0.split(".");
                            let id = aa.nth(0).unwrap_or("?");
                            let act = aa.nth(0).unwrap_or("");

                            if act != "" {
                                if act == "clone" && kv.1 != id {
                                    let l = all.get_mut(&ri.0).unwrap();
                                    let mut vcl = l.select_visual(&("#".to_string() + id)).unwrap().clone();
                                    let vs = l.select_visual(&("#".to_string() + kv.1.as_str())).unwrap();
                                    let fs = ffms.get(&vs.window.resource_id()).unwrap().cur_file.clone();
                                    let f = ffms.remove(&rid).unwrap();
                                    drop(f);
                                    vcl.content = fs;
                                    let nf = Media::new(rid, &mut escope, vcl, drw, drb);
                                    ffms.insert(rid, nf);
                                } else {
                                    match id {
                                        "global"=> {
                                            escope.set_value(act,kv.1.clone());
                                        }
                                        "layer"=> {
                                            let mut files :HashMap<String,String> = nmap!();
                                            loop {
                                                let k = all.keys().nth(0).unwrap().clone();
                                                let l = all.remove(&k).unwrap();
                                                files.insert(k,l.file.clone());

                                                let mut medias = l.select("media");
                                                for m in medias.iter_mut() {
                                                    let idx = m.window.resource_id();
                                                    let f = ffms.remove(&idx).unwrap();
                                                    drop(f.player);
                                                }
                                                l.root_visual.demolish(ctx);
                                                drop(l);

                                                if all.len() == 0 { break }
                                            }

                                            for ln in &self.layers {
                                                let mut file = files.get(ln.0.as_str()).unwrap().clone();
                                                if ln.0 == act {
                                                    file = kv.1.clone() + ".view";
                                                }
                                                let mut nl = Layer::new(file.as_str(), ctx, window,0,0);
                                                nl.fit_all(drw,ctx,&style,width,height);
                                                nl.visibility(true,ctx);
                                                all.insert(ln.0.to_string(),nl);
                                            }
                                            ctx.collect();
                                            rebuild = true;
                                        }
                                        _ => {
                                            let l = all.get_mut(&ri.0).unwrap();
                                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                                            let v = kv.1.as_str();
                                            match act {
                                                "seek-rel" => {
                                                    let d: i64 = i64::from_str(v).unwrap();
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(Player::CTL_SEEK_REL + d).unwrap_or(());
                                                }
                                                "seek-abs" => {
                                                    let d: i64 = i64::from_str(v).unwrap();
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(Player::CTL_SEEK_ABS + d).unwrap_or(());
                                                }
                                                _ => {}
                                            }

                                            match v {
                                                "die" => {
                                                    let f = ffms.remove(&rid).unwrap();
                                                    drop(f);
                                                    let nf = Media::new(rid, &mut escope, vr.clone(), drw, drb);
                                                    ffms.insert(rid, nf);
                                                }/*
                                                "skip-fwd" => {
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(ControlCommand::SkipFwd as isize).unwrap_or(());
                                                }
                                                "skip-bkw" => {
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(ControlCommand::SkipBkw as isize).unwrap_or(());
                                                }
                                                "seek-fwd" => {
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(ControlCommand::SeekFwd as isize).unwrap_or(());
                                                }
                                                "seek-bkw" => {
                                                    ffms.get_mut(&rid).unwrap().player.control_sender.send_blocking(ControlCommand::SeekBkw as isize).unwrap_or(());
                                                }*/
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                XcbEvent::B_UP => {
                    //DomApp::eval(&format!(r#"on_event(eve,"?","b_up",0,0,{})"#,ev.button));
                }
                XcbEvent::NONE => {
                    if nwidth!=width || nheight!=height || rebuild {
                        rebuild = false;
                        width = nwidth;
                        height = nheight;
                        if back_buffer != x::Pixmap::none() {
                            ctx.drop_pixmap(back_buffer);
                        }
                        back_buffer = ctx.new_pixmap(drw, width, height);
                        drb = Drawable::Pixmap(back_buffer);

                        for l in all.iter_mut() {
                            l.1.fit_all(drw, ctx, &style, width, height);
                        }

                        let l = all.get("players").unwrap();
                        let mut medias = l.select("media");

                        for m in medias.iter_mut() {
                            let idx = m.window.resource_id();
                            println!("New media {idx}");
                            ffms.insert(idx, Media::new(idx, &mut escope, m.clone(), drw, drb));
                            thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }

                    for l in all.iter_mut() {
                        let mut icons = l.1.select("i");
                        icons.extend(l.1.select("lbl"));
                        icons.extend(l.1.select("media"));
                        for vi in icons {
                            let vwd = Drawable::Window(vi.window);
                            let vd = Drawable::Pixmap(vi.buf);
                            let gc = ctx.new_gc(drw, 0xFFFFFFFF, 0xFFFFFFFF);
                            if vi.inv_mask != x::Pixmap::none() {
                                let mgc_i = ctx.new_masked_gc(vwd, vi.inv_mask, vi.fg, vi.bg);
                              //  ctx.rect(gc, vwd, 0, 0, vi.width, vi.height);
                                ctx.copy(mgc_i, drb, vd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                                ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                            } else if vi.buf != x::Pixmap::none() {
                            //    ctx.rect(gc, vwd, 0, 0, vi.width, vi.height);
                                ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                            }
                        }
                    }

                    //let gc = ctx.new_gc(drw, 0, 0);
                    //ctx.copy(gc, drb, drw, 0, 0, 0, 0, width, height);
                    ctx.collect();
               //     println!("Collect");
                    //smol::Timer::after(std::time::Duration::from_millis(1));
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                XcbEvent::RESIZE => {
                    if width != ev.width || height != ev.height {
                        nwidth = ev.width;
                        nheight = ev.height;

                        println!("Resizing");
                        loop {
                            let idx = ffms.keys().nth(0).unwrap_or(&0).clone();
                            if idx == 0 { break }
                            let fo = ffms.remove(&idx);
                            if fo.is_some() {
                                let f = fo.unwrap();
                                drop(f);
                            } else {
                                break;
                            }
                        }
                        println!("Resized");
                    }
                }
                XcbEvent::RENDER => {}
                _ => {}
            }
        }
    }
}

impl DomLayer {
    fn new(name:String,file: String)->Self {
        Self {
            name,
            file
        }
    }
    pub fn query(self, sel:&str) ->DomElem {
        DomElem::new(self.name,sel)
    }
}

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
}

impl Rhai {
    pub fn run(&mut self) {
        self.exec("startup();");
    }
    pub fn exec(&mut self, mut script:&str) {
        let mut eval = script!("common","rhai");
        eval += "\n";
        eval += script.trim();
        self.engine.run_with_scope(&mut self.scope,&eval).unwrap();
    }

    fn new()-> Self {
        let mut engine = Engine::new();
        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut engine);
        engine.register_global_module(RandomPackage::new().as_shared_module());

        engine.register_type::<DomApp>().register_fn ("new_app", DomApp::new)
            .register_fn("load_layer",DomApp::load_layer)
            .register_fn("main_loop",DomApp::main_loop);
        let mut scope = RScope::new();
        Self {
            engine,
            scope,
            layers: nmap!()
        }
    }
}