use std::path::Path;
use lazy_static::lazy_static;

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
    layers: domlaymap!()
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
    to_map: Option<x::Pixmap>,
    player: Player,
    events: smol::channel::Receiver<i32>
}

#[derive(Clone)]
struct StreamSettings {
    use_audio: bool,
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
    pub fn new(idx:i32, m: Visual, drw: x::Drawable, drb: x::Drawable) -> Self {
        let ctx = &CTX;
        let settings = StreamSettings {
            use_audio: false,
            speed_factor: 0.125,
            start_secs: -60.0,
            stop_secs: -1.0,
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
            let mut to_rgba_rescaler: Option<Rescaler> = None;
            let mut to_map: Option<x::Pixmap> = None;
            let mcc = m.content.clone();
            let mut input = Option::None;
            let mut flags: i32 = 0;
            loop {
                let file = PathBuf::from(DomApp::eval(&mcc));
                println!("Checking {:?}",file);
                (flags,input) = Player::check(file.clone());

                if (flags & Player::HAS_VIDEO) != 0 {
                    println!("Accepting {:?}",file);
                    break
                }
            }
            let ply = Player::start(input.unwrap(), settings.clone(), move |new_frame| {
                let mut value = to_rgba_rescaler.as_ref();
                let rebuild_rescaler =
                    value.map_or(true, |existing_rescaler| {
                        existing_rescaler.input().format != new_frame.format() //||
                        //existing_rescaler.input().width != new_frame.width() ||
                        //existing_rescaler.input().height != new_frame.height()
                    });
                if rebuild_rescaler {
                    //println!("New scalar {},{}", m.width, m.height);
                    to_rgba_rescaler = Some(rgba_rescaler_for_frame(new_frame, m.width as u32, m.height as u32));
                }

                let rescaler = to_rgba_rescaler.as_mut().unwrap();
                let mut rgb_frame = ffmpeg_next::util::frame::Video::empty();
                rescaler.run(&new_frame, &mut rgb_frame).unwrap();

                let data = rgb_frame.data(0);
                let bytes = data.len();
                let bf = (bytes / (rgb_frame.width() * 4) as usize) as u16;

                if rebuild_rescaler {
                    if to_map.is_some() {
                        ctx.drop_pixmap(to_map.unwrap());
                    }
                    to_map = Some(ctx.new_pixmap(drw, m.width, bf));
                }

                let map = to_map.unwrap();
                let mdrw = Drawable::Pixmap(map);
                let mgc = ctx.new_gc(mdrw, 0xFFFFFFFF, 0x00000000);

                let yofs = (m.height as i16 - bf as i16) / 2;
                ctx.fill(mgc, mdrw, data, 0, 0, m.width, bf);
                ctx.copy(mgc, mdrw, drb, 0, yofs, m.x, m.y, m.width, bf);
                ctx.copy(mgc, mdrw, drw, 0, yofs, m.x, m.y, m.width, bf);
            },sender);

            if ply.is_ok() {
                let player = ply.ok().unwrap();
                return Media {
                    to_map,
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
            layers: nmap!()
        }
    }
    pub fn load_layer(&mut self,name: String,file: String) {
        self.layers.insert(name.clone(),DomLayer::new(name,file));
    }

    fn eval(script: &str) ->String {
        let mut engine = Engine::new();
        //let mut o: String = "".to_string();
        //engine.on_print(|s|{
          //  engine.
            //let mut l = all.root_visual.get_mut("#title").unwrap();
            //let mut v = all.root_visual.select(e).first();
            //v[0].set_content(c)
        //});
        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut engine);
        engine.register_global_module(RandomPackage::new().as_shared_module());
        //engine.register_fn("is_playable",Player::is_playable);
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
        //println!("*********************************** \n{}\n *****************************", eval);
        let res = engine.eval::<String>(&(script!("common","rhai").as_str().to_string() + "\n" + eval)).unwrap();

        res
    }

    fn main_loop(&mut self, iwidth:i64, iheight:i64) {
        let ctx = &CTX;
        let mut nwidth= iwidth as u16;
        let mut nheight = iheight as u16;
        let mut width= 1;
        let mut height = 1;

        let window = ctx.new_window(0xFFFF1010);
        //ctx.prepare(window);
        ctx.show(window);

        let drw = Drawable::Window(window);
        let style = Style::new(drw,&ctx, "common");
        let mut ffms: HashMap<i32,Media> = nmap!();
        let mut all: laymap!() = nmap!();
        for dl in &self.layers {
            all.insert(dl.1.name.clone(),Layer::new(&dl.1.file, ctx, window,0,0,width,height));
        }
        all["overlay"].root_visual.show(ctx);

        let mut back_buffer = ctx.new_pixmap(drw, iwidth as u16, iheight as u16);
        let mut drb = Drawable::Pixmap(back_buffer);

        let mut to_die: Vec<i32> = vec![];
        loop {
            //let mut idx = 0;
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

            if to_die.len()>0 {
                let l = all.get("players").unwrap();
                let mut medias = l.select("media");
                let mut idx:i32 = 0;
                for m in medias.iter_mut() {
                    if to_die.contains(&idx) {
                        let f = ffms.remove(&idx).unwrap();
                        if f.to_map.is_some() { ctx.drop_pixmap(f.to_map.unwrap()) }
                        drop(f);
                        ffms.insert(idx, Media::new(idx,m.clone(), drw, drb));
                    }
                    idx += 1;
                }
            }
            to_die.clear();

            let ev = ctx.wait_event();
            match ev.code {
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

                    let l = &all["overlay"];
                    let mut icons = l.select("i");
                    icons.extend(l.select("lbl"));
                    for vi in icons {
                        let vwd = Drawable::Window(vi.window);
                        let vd = Drawable::Pixmap(vi.buf);
                        let gc = ctx.new_gc(vd, vi.bg, vi.fg);
                        if vi.inv_mask != x::Pixmap::none() {
                            let mgc_i = ctx.new_masked_gc(drw, vi.inv_mask, vi.fg, vi.bg);
                            ctx.rect(gc, vwd, 0, 0, vi.width, vi.height);
                            ctx.copy(mgc_i, drb, vd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                            ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                        } else if vi.buf != x::Pixmap::none() {
                            ctx.copy(gc, vd, vwd, 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }

                    //let gc = ctx.new_gc(drw, 0, 0);
                    //ctx.copy(gc, drb, drw, 0, 0, 0, 0, width, height);
                    ctx.collect();
                    //smol::Timer::after(std::time::Duration::from_millis(1));
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                XcbEvent::RESIZE => {
                    if width != ev.width || height != ev.height {
                        nwidth = ev.width;
                        nheight = ev.height;

                        for idx  in 0..10 {
                            let fo = ffms.remove(&idx);
                            if fo.is_some() {
                                let f = fo.unwrap();
                                if f.to_map.is_some() { ctx.drop_pixmap(f.to_map.unwrap()) }
                                drop(f);
                            }
                        }
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

    fn new(w:u16,h:u16)-> Self {
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