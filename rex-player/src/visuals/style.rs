
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

    fn font_get(&mut self,ctx:&CTX,drw:Drawable,sel: &str,fg: u32,bg: u32,h:u32)->Sprite {
        if !self.fonts.contains_key(sel) {
            let mut file = "fonts/default/".to_string() + h.to_string().as_str();
            let ff = asset!(file,"fnt");
            let mut from = h;
            if !std::fs::exists(ff).unwrap() {
                from = 31;
                file = "fonts/default/31".to_string()
            }
            let sp = Sprite::new(drw, ctx, &file, fg, bg,from,h);
            self.fonts.insert(sel.to_string(), sp);
        }

        self.fonts[sel].clone()
    }

    fn prop_get(&self,sel: &str,prop: &str,def: &str)->String {
        if self.rules.contains_key(sel) {
            let r = &self.rules[sel];
            if r.contains_key(prop) {
                r[prop].to_string()
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