use xcb::render::Color;

struct Style {
    rules: mapmap!(),
    fonts: spritemap!() 
}

impl Style {
    pub fn new(drw:x::Drawable,ctx:&Xcb,file:&str)->Self {
        let raw = style!(file);
        let jdoc = Self::process(&raw);
        let mut dom = json::parse(&jdoc).unwrap();
        let mut rules: mapmap!() = nmap!();

        for m in dom.entries() {
            let mut vals : strmap!() = nmap!();
            for vp in m.1.entries() {
                vals.insert(vp.0.to_string(),vp.1.to_string());
            }
            rules.insert(m.0.to_string(),vals);
        }
        
        let mut fonts : spritemap!() = nmap!();
            
        Self {
            rules,
            fonts
        }
    }

    fn color_from_str(s:&str)->u32 {
        let mut ret = 0;
        if s!="transparent" && s!="0" { ret = 0xFF000000 | u32::from_str_radix(s, 16).unwrap(); }
        ret
    }

    fn apply_sel(&mut self,ctx:&CTX,sel:&str,v:&mut Visual,mw:u16,mh:u16) {
        //println!("Applying: {sel} to {}",v.tag);
        v.fg = Self::color_from_str(&self.prop_get(sel, "fg", &format!("{:x}", v.fg)));
        v.bg = Self::color_from_str(&self.prop_get(sel, "bg", &format!("{:x}", v.bg)));
        v.width = Visual::calc(&self.prop_get(sel, "w", &format!("{}px", v.width)), mw, mh);
        v.height = Visual::calc(&self.prop_get(sel, "h", &format!("{}px", v.height)), mw, mh);
        v.visible = self.prop_get(sel, "visible", v.visible.to_string().as_str()).parse().unwrap();

        if v.checked {
            v.fg = Self::color_from_str(&self.prop_get("@checked", "fg", &format!("{:x}", v.fg)));
            v.bg = Self::color_from_str(&self.prop_get("@checked", "bg", &format!("{:x}", v.bg)));
            v.width = Visual::calc(&self.prop_get("@checked", "w", &format!("{}px", v.width)), mw, mh);
            v.height = Visual::calc(&self.prop_get("@checked", "h", &format!("{}px", v.height)), mw, mh);
            v.visible = self.prop_get("@checked", "visible", v.visible.to_string().as_str()).parse().unwrap();
        }
    }

    fn apply(&mut self,v:&mut Visual,mw:u16,mh:u16) {
        let ctx = &CTX;
        self.apply_sel(ctx,"@root",v,mw,mh);
        self.apply_sel(ctx,v.tag.clone().as_str(),v,mw,mh);

        if v.attrs.contains_key("id") {
            let id = "#".to_string() + v.attrs["id"].as_str();
            self.apply_sel(ctx,&id,v,mw,mh);
        }

        //v.make_assets(Drawable::Window(ctx.master_window), ctx, self);

        if v.visible {
            v.show(ctx);
        } else {
            v.hide(ctx);
        }
    }

    fn font_get(&mut self,ctx:&CTX,drw:Drawable,sel: &str,fg: u32,bg: u32,h:u32)->Sprite {
        let k = (sel.to_string()+h.to_string().as_str()+fg.to_string().as_str()+bg.to_string().as_str());
        if !self.fonts.contains_key(&k) {
            let mut file = "fonts/default/".to_string() + h.to_string().as_str();
            let ff = asset!(file,"fnt");
            let mut from = h;
            if !std::fs::exists(ff).unwrap() {
                from = 81;
                file = "fonts/default/81".to_string()
            }
            let sp = Sprite::new(drw, ctx, &file, fg, bg,from,h);
            self.fonts.insert(k.to_string(), sp);
        }

        self.fonts[&k].clone()
    }

    fn prop_find(&self,v:&Visual,prop: &str,def: &str)->String {
        let mut ret = self.prop_get("@root", prop, def);
        ret = self.prop_get(&v.tag, prop, def);
        if v.attrs.contains_key("id") {
            let id = "#".to_string() + v.attrs["id"].as_str();
            ret = self.prop_get(&id, prop, def);
        }

        ret
    }

    fn prop_get(&self,sel: &str,prop: &str,def: &str)->String {
        if self.rules.contains_key(sel) {
            let r = &self.rules[sel];
            if r.contains_key(prop) {
                let v = r[prop].to_string();
         //       println!("Style: {sel}.{prop}={v}");
                v
            } else {
                def.to_string()
            }
        } else {
            def.to_string()
        }
    }

    fn process(raw:&str)->String {
        let mut pro =  "\"".to_string() + raw.replace("\n"," ").trim() + " ";
        pro = pro.replace(":",": ");
        pro = pro.replace("{","{ ");
        pro = pro.replace("}"," }");
        pro = pro.replace(";","; ");
        while pro.find("  ").is_some() {
            pro = pro.replace("  "," ");
        }
        //pro = pro.replace("\"","\\\"");
        pro = pro.replace("; ","\", \"");
        pro = pro.replace(": ","\": \"");
        pro = pro.replace(" { ","\": { \"");
        pro = pro.replace(", \"} "," }, \"");
        //pro = pro.replace(", \"}","}");
        pro = "{ ".to_string() + &pro[0..pro.len()-3] + " }";
      //  println!("{pro}");
        pro
    }
}