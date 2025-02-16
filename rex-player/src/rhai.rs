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
    layers: domlays!(),
    window: Window,
    drw: Drawable,
    drb: Drawable,
    width: u16,
    height: u16
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

    pub fn new(app:&DomApp,ffms: &mut HashMap<u32,Media>,all: &mut HashMap<String,Layer>, style: &mut Style,idx:u32,scope:&mut RScope,m: Visual, drw: x::Drawable, drb: x::Drawable) -> Self {
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

                for kv in evr {
                    let v = kv.1.clone();
                    let mut aa = kv.0.split(".");
                    let id = aa.nth(0).unwrap_or("?");
                    let act = aa.nth(0).unwrap_or("");

                    app.apply_script_result(scope,ffms,all,style,id,act,v);
                }

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
            layers: vec![],
            window: x::Window::none(),
            drw: x::Drawable::none(),
            drb: x::Drawable::none(),
            width: 1,
            height: 1
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
        //println!("EVAL: {eval}");
        let eval = r#"let eve = result_init();eve = "#.to_string() + eval + r#";return result_complete(eve);"#;

        let rs = engine.eval_with_scope::<String>(scope,&(script!("common","rhai").as_str().to_string() + ";" + eval.as_str())).unwrap().trim().to_string();
        let mut res = rs.split('\n').collect::<Vec<&str>>();
        let mut ret = HashMap::new();
        while res.len()>0 {
            let v = res.pop().unwrap_or("").to_string();
            let k = res.pop().unwrap_or("").to_string();
            if k!="" && v != "" { ret.insert(k,v); }
        }
      //  println!("EVAL RET: {:?}", ret);
        ret
    }

    fn main_loop(&mut self, iwidth:i64, iheight:i64) {
        let mut escope = RScope::new();
        let ctx = &CTX;
        let mut nwidth= iwidth as u16;
        let mut nheight = iheight as u16;
        self.window = ctx.new_window(0xFFFF1010);
        ctx.size(self.window,nwidth,nheight);

        self.drw = Drawable::Window(self.window);
        let mut style = Style::new(self.drw, &ctx, "common");
        let mut ffms: HashMap<u32,Media> = nmap!();
        let mut all: laymap!() = nmap!();
        for dl in &self.layers {
            let mut nl = Layer::new(&dl.1.file, ctx, self.window,0,0);
            nl.visibility(true,ctx);
            all.insert(dl.1.name.clone(),nl);
            ctx.collect();
        }
        ctx.show(self.window);

        let mut back_buffer = ctx.new_pixmap(self.drw, iwidth as u16, iheight as u16);
        self.drb = Drawable::Pixmap(back_buffer);

        let mut to_die: Vec<u32> = vec![];
        escope.push("curlib", "Videos/TV");
        //let mut swap_layers: strmap!() = nmap!();
        let mut rebuild = false;
        loop {
            if !(nwidth!=self.width || nheight!=self.height || rebuild) {
                for f in ffms.iter() {
                    let fr = f.1.events.try_recv();
                    if fr.is_ok() {
                        let idx = *f.0;
                        let ev = fr.unwrap();
                        match ev {
                            Media::ERR | Media::EOF => {
                                println!("Killing {idx}");
                                to_die.push(idx);
                            }
                            _ => {}
                        }
                    }
                }

                if to_die.len() > 0 {
                    for l in all.clone().iter() {
                        let mut medias = l.1.select("media");
                        for m in medias.iter_mut() {
                            let idx = m.window.resource_id();
                            if to_die.contains(&idx) || rebuild {
                                let f = ffms.remove(&idx).unwrap();
                                drop(f);
                                let nf = Media::new(self, &mut ffms, &mut all, &mut style, idx, &mut escope, m.clone(), self.drw, self.drb);
                                ffms.insert(idx, nf);
                            }
                        }
                    }
                    to_die.clear();
                }
            }

            let ev = ctx.wait_event();
            match ev.code {
                XcbEvent::B_DOWN => {
                   /* let rid = ev.window.resource_id();
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
                        for kv in er {
                            let v = kv.1.clone();
                            let mut aa = kv.0.split(".");
                            let id = aa.nth(0).unwrap_or("?");
                            let act = aa.nth(0).unwrap_or("");
                            rebuild = self.apply_script_result(&mut escope,&mut ffms,&mut all,&mut style,id,act,v);
                        }
                    }*/
                }
                XcbEvent::B_UP => {
                    let rid = ev.window.resource_id();
                    let mut res: Vec<(String,HashMap<String,String>)> = vec![];
                    for li in all.iter_mut() {
                        let l = li.1;
                        let vco = l.visual_by_res(rid);
                        if vco.is_some() {
                            let vc = vco.unwrap();
                            if vc.tag=="choices" {
                                let mut cy = ev.y / Visual::DEF_LINE_H as i16;
                                if ev.button!=1||cy<1 { break }
                                cy -= 1;

                                let cx = ev.x*Visual::DEF_CHOICE_COLS as i16/vc.width as i16;
                                let ca = vc.attrs.clone();
                                let blank = "".to_string();
                                let ss = ca.get("selected").unwrap_or(&blank);
                                let n = (cx+cy*Visual::DEF_CHOICE_COLS as i16) as usize;
                                let iss = ca.get("items").unwrap();
                                let cco = iss.split(":").nth(n);
                            //    println!("Choice check:\n {} => {:?} {cx}x{cy} [{n}]",iss,cco);
                                if cco.is_some() {
                                    let cc = cco.unwrap();
                                    if ss!="" {
                                        let sp = ":".to_string() + cc + ":";
                                        let mut sh = ":".to_string() + ss.as_str() + ":";
                                        vc.attrs.remove("selected");
                                        if sh.contains(&sp) {
                                            sh = sh.replace(&sp,":");
                                        } else {
                                            sh = sh + cc + ":";
                                        }
                                        if sh==":" {
                                            vc.attrs.insert("selected".to_string(), "".to_string());
                                        } else {
                                            vc.attrs.insert("selected".to_string(), sh[1..sh.len() - 1].to_string());
                                        }
                                        println!("New selected: {ss}:{cc}");
                                    } else {
                                        vc.attrs.remove("selected");
                                        vc.attrs.insert("selected".to_string(),cc.to_string());
                                        println!("First selected: {cc}");
                                    }
                                    let v = Self::eval(&mut escope, &vc.content.clone());
                                    vc.set_content(self.drw, ctx, &mut style, v["_"].as_str());
                                    ctx.request_redraw(vc.window,0,0,vc.width,vc.height);
                                }
                            } else {
                                res.push((li.0.clone(), Self::eval(&mut escope, &format!(r#"on_event(eve,"{}","b_up",0,0,{})"#, vc.attrs.get("id").unwrap_or(&"?".to_string()), ev.button))));
                            }
                        }
                    }
                    for ri in res {
                        let er = ri.1;
                        for kv in er {
                            let v = kv.1.clone();
                            let mut aa = kv.0.split(".");
                            let id = aa.nth(0).unwrap_or("?");
                            let act = aa.nth(0).unwrap_or("");

                            let mr = self.apply_script_result(&mut escope,&mut ffms,&mut all,&mut style,id,act,v);
                            if mr { rebuild = mr }
                        }
                    }
                    //println!("Btn up done {:?}",ffms);
                }
                XcbEvent::NONE => {
                    if nwidth!=self.width || nheight!=self.height || rebuild {
                        println!("Rebuild");
                        rebuild = false;
                        self.width = nwidth;
                        self.height = nheight;
                        if back_buffer != x::Pixmap::none() {
                            ctx.drop_pixmap(back_buffer);
                        }
                        back_buffer = ctx.new_pixmap(self.drw, self.width, self.height);
                        self.drb = Drawable::Pixmap(back_buffer);

                        for l in all.iter_mut() {
                            l.1.fit_all(self.drw, ctx, &mut style, self.width, self.height);
                        }

                        for l in all.iter_mut() {
                            let lc = l.1.clone();
                            let mut on_loaders = lc.select("choices");
                            for m in on_loaders {
                                let sc = Self::eval(&mut escope, &m.content.clone());
                                let uv = l.1.visual_by_res(m.window.resource_id()).unwrap();
                                uv.set_content(self.drw, ctx, &mut style, &sc["_"].clone());
                                if m.attrs.contains_key("id") {
                                    escope.set_value(m.attrs["id"].to_owned() + "_selected","");
                                }
                            }
                        }

                        let mut l = all.get_mut("players").cloned().unwrap();
                        let mut medias = l.select("media");
                        for m in medias.iter_mut() {
                            let idx = m.window.resource_id();
                            println!("New media {}x{}",m.width,m.height);
                            let med = Media::new(self,&mut ffms,&mut all,&mut style,idx, &mut escope, m.clone(), self.drw, self.drb);
                            ffms.insert(idx, med);
                            thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }

                    let cfgs = format!("{:x}", Visual::DEF_SEL_BG);
                    let cfg = u32::from_str_radix(&style.prop_get(":selected", "fg", &cfgs), 16).unwrap();

                    for l in all.iter_mut() {
                        let mut icons = l.1.select("i");
                        icons.extend(l.1.select("lbl"));
                        icons.extend(l.1.select("media"));
                        icons.extend(l.1.select("choices"));
                        for vi in icons {
                            let vwd = Drawable::Window(vi.window);
                            let vd = Drawable::Pixmap(vi.buf);
                            if vi.tag == "media" {
                                let gc = ctx.new_gc(self.drw, 0, 0);
                                ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                            } else if vi.inv_mask != x::Pixmap::none() && vi.mask != x::Pixmap::none() {
                                let mgc_i = ctx.new_masked_gc(vwd, vi.inv_mask, vi.fg, vi.bg);
                                ctx.copy(mgc_i, self.drb, vwd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                                if vi.checked {
                                    let gc = ctx.new_gc(self.drw, cfg, vi.bg);
                                    ctx.rect(gc, vwd, 1, 1, vi.width-2, vi.height-2);
                                }
                                let mgc = ctx.new_masked_gc(vwd, vi.mask, vi.fg, vi.bg);
                                ctx.copy(mgc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                            }
                        }
                    }

                    ctx.collect();
                    thread::sleep(std::time::Duration::from_millis(1));
                }
                XcbEvent::RESIZE => {
                    if self.width != ev.width || self.height != ev.height {
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
                XcbEvent::RENDER => {
                    for l in all.iter_mut() {
                        let vo= l.1.visual_by_res(ev.window.resource_id());
                        if vo.is_some() {
                            let vi = vo.unwrap();

                            let vd = Drawable::Pixmap(vi.buf);
                            let vwd = Drawable::Window(vi.window);
                            let gc = ctx.new_gc(vd,vi.fg,vi.bg);
                            ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn apply_script_result(&self,escope: &mut RScope,ffms:&mut HashMap<u32,Media>,all: &mut HashMap<String, Layer>,style: &mut Style, id: &str, act: &str, v: String)->bool {
        if id=="_" && v=="?" { return false }
        let mut ret = false;
        let mut more: Vec<HashMap<String,String>> = vec![];

        println!("{id}->{act}={v}");
        let ctx = &CTX;
        match id {
            "globals"=> {
                escope.set_value(act,v);
            }
            "layers"=> {
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
                    print!(".");
                    if all.len() == 0 { break }
                }

                for ln in &self.layers {
                    let mut file = files.get(ln.0.as_str()).unwrap().clone();
                    if ln.0 == act {
                        file = v.clone() + ".view";
                    }
                    let mut nl = Layer::new(file.as_str(), ctx, self.window,0,0);
                    nl.fit_all(self.drw,ctx,style,self.width,self.height);
                    nl.visibility(true,ctx);
                    all.insert(ln.0.to_string(),nl);
                }
                ctx.collect();
                println!("Layers changed");
                return true;
            }
            _ => {
                match act {
                    "clone" => {
                        let l = all.get_mut("players").unwrap();
                        let vs = l.select_visual(&("#".to_string() + v.as_str())).unwrap().clone();
                        let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                        let rid = vr.window.resource_id();

                        let fs = ffms.get(&vs.window.resource_id()).unwrap().cur_file.clone();

                        let f = ffms.remove(&rid).unwrap();
                        drop(f);
                        vr.content = fs;
                        let mv = vr.clone();
                        let nf = Media::new(self,ffms,all,style,rid,escope,mv,self.drw, self.drb);
                        ffms.insert(rid, nf);
                        return true;
                    }
                    _ => {}
                }
                match v.as_str() {
                    "die" => {
                        if act=="q*" {
                            let l = all.get_mut("players").unwrap().clone();
                            let medias = l.select("media");
                            for vr in medias {
                                if &vr.attrs.get("id").unwrap_or(&"".to_string())[0..1]=="q" {
                                    let rid = vr.window.resource_id();
                                    let f = ffms.remove(&rid).unwrap();
                                    drop(f);
                                    let mv = vr.clone();
                                    let nf = Media::new(self, ffms, all, style, rid, escope, mv, self.drw, self.drb);
                                    ffms.insert(rid, nf);
                                }
                            }
                        } else {
                            let l = all.get_mut("players").unwrap();
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let rid = vr.window.resource_id();
                            let f = ffms.remove(&rid).unwrap();
                            drop(f);
                            let mv = vr.clone();
                            let nf = Media::new(self, ffms, all, style, rid, escope, mv, self.drw, self.drb);
                            ffms.insert(rid, nf);
                        }
                    }
                    _ => {}
                }
                for li in all.iter_mut() {
                    let l = li.1;
                    let lk = li.0;
                    if l.select_visual(&("#".to_string() + id)).is_none() { continue }

                    match act {
                        "checked" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            if v == "!" {
                                vr.checked = !vr.checked;
                            } else {
                                vr.checked = bool::from_str(&v).unwrap();
                            }
                            vr.show(ctx);
                        }
                        "visible" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            if v == "!" {
                                vr.visible = !vr.visible;
                            } else {
                                vr.visible = bool::from_str(&v).unwrap();
                            }
                            if vr.visible { vr.show(ctx) }
                            else {
                                vr.hide(ctx);
                                if vr.tag=="choices" && vr.attrs.contains_key("id") {
                                    let id = &vr.attrs["id"].to_owned();
                                    let blank = "".to_string();
                                    let lsel = vr.attrs.get("__last-selected").unwrap_or(&blank).clone();
                                    let sel = vr.attrs.get("selected").unwrap_or(&blank).clone();
                                    if lsel!=sel  {
                                        escope.set_value(id.to_owned() + "_selected", sel.clone());
                                        more.push(DomApp::eval(escope,&format!(r#"on_event(eve,"{id}_selected","change",0,0,0)"#)));
                                        if vr.attrs.contains_key("__last-selected") { vr.attrs.remove("__last-selected"); }
                                        vr.attrs.insert("__last-selected".to_string(),sel.clone());
                                    }
                                }
                            }
                        }
                        "content" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            vr.set_content(self.drw, ctx, style, &v);
                        }
                        "seek-rel" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let d: i64 = i64::from_str(&v).unwrap();
                            ffms.get_mut(&vr.window.resource_id()).and_then(|f|f.player.control_sender.send_blocking(Player::CTL_SEEK_REL + d).ok()).unwrap_or(());
                        }
                        "seek-abs" => {
                            let vr = l.select_visual(&("#".to_string() + id)).unwrap();
                            let d: i64 = i64::from_str(&v).unwrap();
                            ffms.get_mut(&vr.window.resource_id()).and_then(|f|f.player.control_sender.send_blocking(Player::CTL_SEEK_ABS + d).ok()).unwrap_or(());
                        }
                        _ => {}
                    }
                }
                if more.len()>0 {
                    for mr in more {
                        for ei in mr {
                            let mut ia = ei.0.split(".");
                            let act = ia.nth(0).unwrap_or("").to_string();
                            let id = ia.nth(0).unwrap_or("").to_string();
                            self.apply_script_result(escope, ffms, all, style, &id, &act, ei.1.clone());
                        }
                    }
                }
            }
        }
        println!("No rebuild");
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