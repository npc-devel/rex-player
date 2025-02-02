

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

impl DomApp {
    fn new() ->Self {
        Self {
            layers: nmap!()
        }
    }
    pub fn load_layer(&mut self,name: String,file: String) {
        self.layers.insert(name.clone(),DomLayer::new(name,file));
    }

    fn eval(&mut self, ctx:&Xcb, style:&Style,  all: &mut laymap!(), script: &str) ->String {
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
        let lines = res.split("\n");
        let res = lines.clone().last().unwrap().to_owned();
        for line in lines {
            if line.starts_with("->") {
                //println!("{line}");
                let sa = line[2..].split(":").collect::<Vec<&str>>();
                all.get_mut(sa[1]).unwrap().root_visual.get_mut(sa[3]).unwrap().set_content(ctx,style,sa[5]);
            }
        }
        res
    }

    fn main_loop(&mut self,iwidth:i64,iheight:i64) {
        let mut width= iwidth as u16;
        let mut height = iheight as u16;

        let mut vctx = Xcb::new();
        let mut ctx = &mut vctx;
        let mut back_buffer = x::Pixmap::none();
        let window = ctx.new_window(0xFF101010);
        ctx.prepare(window);
        ctx.show(window);

        let style = Style::new(&ctx, "common");
        let mut ffms: Vec<(x::Drawable, Player)> = vec![];
        let mut li = 0;

        let mut all: laymap!() = nmap!();
        for dl in &self.layers {
            all.insert(dl.1.name.clone(),Layer::new(&dl.1.file, &mut ctx, window,0,0,width,height));
        }
        all["overlay"].root_visual.show(ctx);
        //layers.
        //layers.all.insert("overlay".to_string(),Layer::new("osd.view", &mut ctx, self.window,0,0,self.width,self.height));
        loop {
            //  self.

            let ev = ctx.wait_event();
            li += 1;
            match ev.code {
                XcbEvent::NONE => {
                    let mut idx = 0;
                    let bbw = Drawable::Window(window);
                    let bbd = Drawable::Pixmap(back_buffer);
                    ctx.copy(ctx.gc, bbd, bbw, 0, 0, 0, 0, width, height);

              //      let l = all.get("players").unwrap().clone();
                   /* let mut needs_fit: bool = false;
                    for mut f in ffms.iter_mut() {
                            let m = l.select("media")[idx];
                            if f.1.dst != x::Drawable::none() {
                                ctx.copy(ctx.gc, f.1.dst, bbd, 0, 0, m.x, m.y, m.width, m.height);
                            }
                        }
                        idx += 1;
                    }*/
                //    if needs_fit {
                  //      all.get_mut("overlay").unwrap().fit_all(ctx,&style,width,height);
                   // } else {
                   //     let bbw = Drawable::Window(window);

                        let l = &all["overlay"];
                        let mut icons = l.select("i");
                        icons.extend(l.select("lbl"));
                        for vi in icons {
                            let wd = Drawable::Window(vi.window);

                            if vi.inv_mask != x::Pixmap::none() {
                                let vd = Drawable::Pixmap(vi.buf);
                                let gc = ctx.new_gc(vd, vi.bg, vi.fg);
                                let mgc = ctx.new_masked_gc(wd, vi.mask, vi.fg, vi.bg);
                                let mgc_i = ctx.new_masked_gc(wd, vi.inv_mask, vi.fg, vi.bg);

                                ctx.rect(gc, wd, 0, 0, vi.width, vi.height);
                                ctx.copy(mgc_i, bbd, wd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                                ctx.copy(mgc, vd, wd, 0, 0, 0, 0, vi.width, vi.height);
                            } else if vi.buf != x::Pixmap::none() {
                                ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), wd, 0, 0, 0, 0, vi.width, vi.height);
                            }
                        }

                        ctx.collect();
                        thread::sleep(Duration::from_millis(1));
                    //}
                }
                XcbEvent::RESIZE => {
                    if width != ev.width || height != ev.height {
                        //     println!("RESIZE {}x{}",ev.width,ev.height);
                        width = ev.width;
                        height = ev.height;

                        back_buffer = ctx.new_pixmap(width, height);

                        //ctx.map_bg(window, back_buffer);

                        //ctx.map_bg(self.window,s);
                         for l in all.iter_mut() {
                             l.1.fit_all(ctx,&style,width,height);
                         }
                        //all_layers.get_mut("overlay").unwrap().fit_all(ctx,&self.style,self.width,self.height);
                        //players.fit_all(ctx,&self.style,self.width,self.height);
                        let l = all.get("players").unwrap().clone();
                        let medias = l.select("media");
                        //     println!("{:?}",medias.len());
                        if ffms.is_empty() {
                            for m in medias {
                                loop {
                                    let file = &self.eval(ctx,&style,&mut all,&m.content);
                                    let mx = m.x;
                                    let my = m.y;
                                    let mw = m.width;
                                    let mh = m.height;

                                    let dst = ctx.new_pixmap(mw as u16, mh as u16);
                                    let ply = Player::start(PathBuf::from(file),move|frame,sco|{
                                            let mut scc = sco.as_mut().unwrap();

                                            let mut rgb_frame = Video::empty();
                                            scc.scalar.run(&frame, &mut rgb_frame).unwrap();
                                            let data = rgb_frame.data(0);
                                            let pl = (data.len() as u32/4) as u16;
                                            scc.rh = rgb_frame.plane_height(0) as u16;
                                            scc.rw = (pl/scc.rh) as u16;

                                            let expected_bytes = (4*mw as u32*mh as u32) as usize;
                                            let csd: &[u8] = bytemuck::cast_slice(&data[0..expected_bytes]);
                                            //csd[0..expected_bytes].copy_from_slice(&csd[0..]);
                                            //csd[expected_bytes..].fill(u8::EQUILIBRIUM);
                                            let bbd = Drawable::Pixmap(back_buffer);
                                            let sbd = Drawable::Pixmap(scc.dst);
                                            let gc = scc.ctx.new_gc(bbd, mw as u32, mh as u32);
                                            scc.ctx.fill(gc, scc.drw, csd, 0, 0, mw, mh);
                                            scc.ctx.copy(gc, sbd, bbd, 0, 0, mx, my, mw, mh);

                                    },|isplaying|{},mw as u32,mh as u32,dst);
                                    if ply.is_ok() {
                                        ffms.push((Drawable::Window(m.window), ply.ok().unwrap()));
                                        break;
                                    }
                                }
                            }
                        } else {
                            let mut idx = 0;
                            for m in medias {
                                let fo = ffms.get_mut(idx);
                                if fo.is_some() {
                                    //fo.unwrap().1.rescale(m.width as u32, m.height as u32);
                                }
                                idx += 1;
                            }
                        }

                        /*    self.ffms.clear();

                            let medias  = self.players.select("media");
                            for m in medias {
                                self.ffms.push((Drawable::Window(m.window),FfMpeg::new(ctx, &asset!("loader","mp4"), m.width as u32,m.height as u32)));
                            }*/

                        // self.players.anchor_fit_to(ctx, 0, 0, self.width, self.height);
                        /*   let bb = &mut self.players.controls.get(&medias[0]).unwrap().buf.resource_id();
                           for r in senders.iter() {
                               r.send(format!("buf={bb} {} {}",self.width,self.height)).unwrap();
                           }*/
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
        /*if script.starts_with("??=") {
            let l = script.len();
            script = &script[3..l - 2];
        }*/
        self.engine.run_with_scope(&mut self.scope,&eval).unwrap();
    }

    fn new(w:u16,h:u16)-> Self {
        let mut engine = Engine::new();
        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut engine);
        engine.register_global_module(RandomPackage::new().as_shared_module());

        let mut rec: Option::<Receiver<String>> = Option::None;
        engine.register_type::<DomApp>().register_fn ("new_app", DomApp::new)
            .register_fn("load_layer",DomApp::load_layer)
            .register_fn("main_loop",DomApp::main_loop);
        let mut scope = RScope::new();
      //  scope.push("app","new_app()");

        Self {
            /*width:w,
            height:h,
            ctx,
            window,
            back_buffer,
            style*/
            engine,
            scope,
            layers: nmap!()
        }
    }
    
    fn clean_up(&mut self) {

    }
    
    fn prepare(&mut self) {
        //FfMpeg::static_init();
        //self.ctx.show(self.window);
    }
    //fn idle(&self) {
        //self.ctx.collect();
        //thread::sleep(Duration::from_millis(1));
    //}

    /*fn main_loop(&mut self) {
        let mut ffms: Vec<(x::Drawable,FfMpeg)> = vec![];
        let mut ctx = &self.ctx;
        let mut li = 0;

        //layers.all.insert("players".to_string(),Layer::new("media-quad.view", &mut ctx, self.window,0,0,self.width,self.height));
        //layers.all.insert("overlay".to_string(),Layer::new("osd.view", &mut ctx, self.window,0,0,self.width,self.height));
        loop {
          //  self.all["overlay"].root_visual.show(ctx);

            let ev = ctx.wait_event();
            li+=1;
            match ev.code {
                XcbEvent::NONE => {
                    let mut idx = 0;
                    let bbd = Drawable::Pixmap(self.back_buffer);
                    //let l = &self.all["players"];
                    for mut f in ffms.iter_mut() {
                        if f.1.wait_events(ctx) {
                      //      let m = l.select("media")[idx];
                        //    if f.1.dst != x::Drawable::none() {
                          //      ctx.copy(ctx.gc, f.1.dst, bbd, 0, 0, m.x, m.y, m.width, m.height);
                           // }
                        } else {
                         //   let m = l.select("media")[idx];
                            loop {
                                //let file = self.exec(&m.content).clone();
                                //let inp = FfMpeg::open(&file);
                                //if inp.is_ok() {
                                  //  f.1 = FfMpeg::new(inp.unwrap(), m.width as u32, m.height as u32);
                                   // break;
                                //}
                            }
                        }
                        idx += 1;
                    }
                    let bbw = Drawable::Window(self.window);
                    ctx.copy(ctx.gc, bbd, bbw, 0, 0, 0, 0, self.width, self.height);
                    //let l = &self.all["overlay"];
                    //let mut icons = l.select("i");
                    //icons.extend(l.select("lbl"));
                    /*for vi in icons {
                        let wd = Drawable::Window(vi.window);

                        if vi.inv_mask != x::Pixmap::none() {
                            let vd = Drawable::Pixmap(vi.buf);
                            let gc = ctx.new_gc(vd, vi.bg, vi.fg);
                            let mgc = ctx.new_masked_gc(wd, vi.mask, vi.fg, vi.bg);
                            let mgc_i = ctx.new_masked_gc(wd, vi.inv_mask, vi.fg, vi.bg);

                            ctx.rect(gc, wd, 0, 0, vi.width, vi.height);
                            ctx.copy(mgc_i, bbd, wd, vi.ax, vi.ay, 0, 0, vi.width, vi.height);
                            ctx.copy(mgc, vd, wd, 0, 0, 0, 0, vi.width, vi.height);
                        } else if vi.buf != x::Pixmap::none() {
                            ctx.copy(ctx.gc, Drawable::Pixmap(vi.buf), wd, 0, 0, 0, 0, vi.width, vi.height);
                        }
                    }*/

                    self.idle();
                }
                XcbEvent::RESIZE => {
                    if self.width!=ev.width || self.height!=ev.height {
                        //     println!("RESIZE {}x{}",ev.width,ev.height);
                        self.width = ev.width;
                        self.height = ev.height;

                        self.back_buffer = ctx.new_pixmap(self.width,self.height);
                        ctx.map_bg(self.window,self.back_buffer);

                        //ctx.map_bg(self.window,s);
                       /* for l in self.all.iter_mut() {
                            l.1.fit_all(ctx,&self.style,self.width,self.height);
                        }*/
                        //all_layers.get_mut("overlay").unwrap().fit_all(ctx,&self.style,self.width,self.height);
                        //players.fit_all(ctx,&self.style,self.width,self.height);
                        /*let l = &self.all["players"];
                        let medias = l.select("media");
                        //     println!("{:?}",medias.len());
                        if ffms.is_empty() {
                            for m in medias {
                                loop {
                                    let file = &self.exec(&m.content);
                                    let inp = FfMpeg::open(file);
                                    if inp.is_ok() {
                                        ffms.push((Drawable::Window(m.window), FfMpeg::new(inp.unwrap(), m.width as u32, m.height as u32)));
                                        break;
                                    }
                                }
                            }
                        } else {
                            let mut idx = 0;
                            for m in medias {
                                let fo = ffms.get_mut(idx);
                                if fo.is_some() {
                                    fo.unwrap().1.rescale(m.width as u32, m.height as u32);
                                }
                                idx += 1;
                            }
                        }*/
                        /*    self.ffms.clear();

                            let medias  = self.players.select("media");
                            for m in medias {
                                self.ffms.push((Drawable::Window(m.window),FfMpeg::new(ctx, &asset!("loader","mp4"), m.width as u32,m.height as u32)));
                            }*/

                        // self.players.anchor_fit_to(ctx, 0, 0, self.width, self.height);
                        /*   let bb = &mut self.players.controls.get(&medias[0]).unwrap().buf.resource_id();
                           for r in senders.iter() {
                               r.send(format!("buf={bb} {} {}",self.width,self.height)).unwrap();
                           }*/
                    }
                }
                XcbEvent::RENDER => {

                }
                _ => {}
            }
        }
    }*/
}