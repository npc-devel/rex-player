
struct Style {
    rules: mapmap!(),
    fonts: spritemap!() 
}

impl Style {
    pub fn new(ctx:&Xcb,file:&str)->Self {
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
        fonts.insert("_".to_string(),Sprite::new(ctx,"fonts/default/21",0xFFFFFFFF,0xFF000000));
            
        Self {
            rules,
            fonts
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