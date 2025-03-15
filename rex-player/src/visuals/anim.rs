use std::fmt::Pointer;


struct Anim {
    begun: bool,
    over: bool,
    start_sel: String,
    end_sel: String,
    started: Instant,
    ends: Instant,
    target: u32,
    transition: u64
}

impl Anim {
    pub fn new(style:&Style,start_sel:&str,end_sel:&str,target:u32)->Self {
        let transition = u64::from_str(&style.prop_get(end_sel,"transition","1000")).unwrap();
        let started = Instant::now();
        let ends = started + Duration::from_millis(transition);
        Self {
            begun: false,
            over: false,
            start_sel: start_sel.to_string(),
            end_sel: end_sel.to_string(),
            started,
            ends,
            target,
            transition
        }
    }
    pub fn reset(&mut self,style:&Style,target:u32) {
        self.target = target;
        let started = Instant::now();
        let ends = started + Duration::from_millis(self.transition);
        if self.begun && !self.over {
            self.started = started;
            self.ends = ends;
        } else {
            self.begun = false;
            self.over = false;
            self.started = started;
            self.ends = ends;
        }
    }

    pub fn apply_end(&mut self,w: x::Window,all:&mut laymap!(),style:&mut Style) {
        let ctx = &CTX;
        self.over = true;
        if &self.end_sel[0..1] == "@" {
            let mut each = style.prop_get(&self.end_sel,"each","*");
            let nt = &each[0..1]=="!";
            if nt { each = each[1..].to_string(); }
            let sp = format!(",{each},");
            let c = style.prop_get(&self.end_sel, "cursor-visible", "");
            
            match c.as_str() {
                "true"=> {
                    ctx.cursor_vis(true);
                }
                "false" => {
                    ctx.cursor_vis(false);
                }
                _=> {}
            }
            let dt = each.contains("@target");
            //println!("DTO: {dt} {}",self.target);
            for l in all.iter_mut() {
                let mut la = l.1.clone();
                for v in la.select("*") {
                    let rid = v.window.resource_id();

                    let sv = (",".to_string() + v.tag.as_str() + ",");
                    if (!nt && sp.contains(&sv))
                        || (nt && !sp.contains(&sv))
                        || (dt && rid==self.target && self.target!=0) {
                        let vr = l.1.visual_by_res(v.window.resource_id()).unwrap();
                        style.apply_sel(ctx,&self.end_sel,vr,vr.pwidth,vr.pheight);

                        vr.make_assets(Drawable::Window(ctx.master_window), ctx, style);
                        if vr.visible {
                            vr.show(ctx);
                        } else {
                            vr.hide(ctx);
                        }
                 //       println!("Anim over: {}",self.end_sel);
                    }
                }
            }

        }
    }

    pub fn apply_start(&mut self,w: x::Window,all:&mut laymap!(),style:&mut Style) {
        let ctx = &CTX;
        self.begun = true;
        if &self.start_sel[0..1]=="@" {
            let mut each = style.prop_get(&self.start_sel,"each","*");
            let nt = &each[0..1]=="!";
            if nt { each = each[1..].to_string(); }
            let sp = format!(",{each},");
            let c = style.prop_get(&self.start_sel, "cursor-visible", "");
            
            match c.as_str() {
                "true"=> {
                    ctx.cursor_vis(true);
                }
                "false" => {
                    ctx.cursor_vis(false);
                }
                _=> {}
            }
            let dt = each.contains("@target");
         //   println!("DTS: {dt} {}",self.target);
            for l in all.iter_mut() {
                let mut la = l.1.clone();
                for v in la.select("*") {
                    let rid = v.window.resource_id();

                    let sv = (",".to_string() + v.tag.as_str() + ",");
                    if  (!nt && sp.contains(&sv))
                        || (nt && !sp.contains(&sv))
                        || (dt && rid==self.target && self.target!=0) {
                        let vr = l.1.visual_by_res(v.window.resource_id()).unwrap();
                        style.apply_sel(ctx,&self.start_sel,vr,vr.pwidth,vr.pheight);

                        vr.make_assets(Drawable::Window(ctx.master_window), ctx, style);
                        if vr.visible {
                            vr.show(ctx);
                        } else {
                            vr.hide(ctx);
                        }
           //             println!("Anim started: {}",self.start_sel);
                    }
                }
            }

        }
    }

    pub fn update(&mut self,w: x::Window,all:&mut laymap!(),style:&mut Style,now:Instant)->bool {
        let ctx = &CTX;

        if self.started <= now && self.ends > now {
            /*let delta = now - self.started;
            let range = self.ends - self.started;
            let progress = delta.as_millis() as f64/range.as_millis() as f64;*/
            if(!self.begun) {
                self.apply_start(w,all,style);
            }
            return true;
        } else if self.begun {
            if !self.over {
                self.apply_end(w,all,style);
                return true;
            }
        }
        false
    }
}
struct Timeline {
    aa: Vec<String>,
    start: Instant,
    keyframes: HashMap<Instant,Vec<String>>
}

/*impl Timeline {
    pub fn new()->Self {
        Self {
            aa: vec![],
            start: std::time::Instant::now(),
            keyframes: HashMap::new()
        }
    }

    pub fn update(&mut self) {
        for a in self.aa.iter_mut() {

        }
    }

    pub fn purge(&mut self,uid:&str) {
        let sp = format!(":{uid}");
        for k in self.keyframes.iter_mut() {
            let mut idx = 0;
            for a in k.1.iter_mut() {
                if a.contains(&sp) {
                    k.1.remove(idx);
                }
                idx += 1;
            }
        }
    }

    pub fn assert(&mut self,style:&Style,start_sel:&str,end_sel:&str,uid:&str) {
        if uid !="" { self.purge(uid) }

        let tr = style.prop_get(end_sel,"transition","1000");
        let ins = Instant::now() + Duration::from_millis(u64::from_str(&tr).unwrap());

        let ad = format!("{:?}@{start_sel}->{end_sel}:{uid}",ins);
        self.aa.push(ad);

        /*if self.keyframes.contains_key(&ins) {
           self.keyframes.get_mut(&ins).unwrap().push(format!("{start_sel}->{end_sel}:{uid}"));
        } else {
            self.keyframes.insert(ins.clone(),vec![format!("{start_sel}->{end_sel}:{uid}")]);
        }*/
    }
}*/