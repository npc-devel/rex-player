use std::path::Path;
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
    const POS_START:i32 = 1000;
    pub fn new(idx:i32, m: Visual, drw: x::Drawable, drb: x::Drawable) -> Self {
        let ctx = &CTX;
        let settings = StreamSettings {
            use_audio: false,
            frame_skip: 0,
            speed_factor: 0.1,
            start_secs: -120.0,
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
            let mcc = m.content.clone();
            let mut input = Option::None;
            let mut flags: i32 = 0;
            let mut evr = HashMap::new();
            loop {
                evr = DomApp::eval(&mcc);
                let file = PathBuf::from(&evr["_"]);
                println!("Checking {:?}",file);
                (flags,input) = Player::check(file.clone());

                if (flags & Player::HAS_VIDEO) != 0 {
                    println!("Accepting {:?}",file);
                    break
                }
            }
            let ply = Player::start(&m,drw,drb,input.unwrap(), settings.clone(), sender);
            if ply.is_ok() {
                let player = ply.ok().unwrap();
                return Media {
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

    fn eval(script: &str) ->HashMap<String,String> {
        let mut engine = Engine::new();
        //let mut o: String = "".to_string();
        //engine.on_print(|s|{
          //  engine.
            //let mut l = all.root_visual.get_mut("#title").unwrap();
            //let mut v = all.root_visual.select(e).first();
            //v[0].set_content(c)
        //});
        let fs = FilesystemPackage::new();
        //let ser = serde_json::Serializer::new(());
        fs.register_into_engine(&mut engine);
        //engine.register_global_module(ser.as_shared_module());
        engine.register_global_module(RandomPackage::new().as_shared_module());
        engine.register_fn("replace_layer",Self::replace_layer);
        //let mut l = all.get_mut(l).unwrap();
        //let mut v = l.root_visual.select(e);

       // engine.register_fn("content",|l: &str, e: &str, c: &str| {
            //let mut l = all.get(l).unwrap();
         //   let mut v = all.root_visual.select(e).first();
            //v[0].set_content(c);
        //});
        let mut eval = script.trim();
        if eval.starts_with("??=") {
            let l = eval.len();
            eval = &script[3..l-2];
            //            println!("*********************************** \n{}\n *****************************", script);
            //
        }
        let eval = r#"let eve = result_init();let pr = "#.to_string() + eval + r#";eve = result(eve,"_",pr);return result_complete(eve);"#;

        //println!("*********************************** \n{}\n *****************************", eval);
        let rs = engine.eval::<String>(&(script!("common","rhai").as_str().to_string() + ";" + eval.as_str())).unwrap().trim().to_string();
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
        let ctx = &CTX;
        let mut nwidth= iwidth as u16;
        let mut nheight = iheight as u16;
        let mut width= 1;
        let mut height = 1;

        let window = ctx.new_window(0xFFFF1010);
        ctx.size(window,nwidth,nheight);

        let drw = Drawable::Window(window);
        let style = Style::new(drw,&ctx, "common");
        let mut ffms: HashMap<i32,Media> = nmap!();
        let mut all: laymap!() = nmap!();
        for dl in &self.layers {
            let mut nl = Layer::new(&dl.1.file, ctx, window,0,0,width,height);
            nl.visibility(true,ctx);
            all.insert(dl.1.name.clone(),nl);
            ctx.collect();
        }
        ctx.show(window);

        let mut back_buffer = ctx.new_pixmap(drw, iwidth as u16, iheight as u16);
        let mut drb = Drawable::Pixmap(back_buffer);

        let mut to_die: Vec<i32> = vec![];
        loop {
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
                let mut idx:i32 = 0;
                for m in medias.iter_mut() {
                    if to_die.contains(&idx) {
                        let f = ffms.remove(&idx).unwrap();
                        drop(f);
                        let nf = Media::new(idx, m.clone(), drw, drb);
                        ffms.insert(idx, nf);
                    }
                    idx += 1;
                }
                to_die.clear();
            }


            //println!("Fetch ev");
            let ev = ctx.wait_event();
            //println!("{:?}",ev);
            match ev.code {
                XcbEvent::B_DOWN => {
                    let er = DomApp::eval(&format!(r#"on_event(eve,"?","b_down",0,0,{})"#,ev.button));
                    ffms.get_mut(&0).unwrap().player.control_sender.send_blocking(ControlCommand::SkipFwd).unwrap();
                }
                XcbEvent::B_UP => {
                    DomApp::eval(&format!(r#"on_event(eve,"?","b_up",0,0,{})"#,ev.button));
                }
                XcbEvent::NONE => {
                    if nwidth!=width || nheight!=height {
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
                        let mut idx: i32 = 0;
                        for m in medias.iter_mut() {
                            println!("New media {idx}");
                            ffms.insert(idx, Media::new(idx,m.clone(), drw, drb));
                            thread::sleep(std::time::Duration::from_millis(10));
                            idx += 1;
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
                            let fo = ffms.remove(&0);
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