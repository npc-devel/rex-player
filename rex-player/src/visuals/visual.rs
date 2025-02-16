#[derive(Clone, CustomType,Debug)]
pub struct Visual {
    x:i16,
    y:i16,
    iax:i16,
    iay:i16,
    ax:i16,
    ay:i16,
    width:u16,
    height:u16,
    lx:i16,
    ly:i16,
    lwidth:u16,
    lheight:u16,
    pwidth:u16,
    pheight:u16,
    inv_mask: x::Pixmap,
    mask: x::Pixmap,
    buf: x::Pixmap,
    window: x::Window,
    bg: u32,
    fg: u32,
    children: Vec<Visual>,
    attrs: strmap!(),
    tag: String,
    content: String,
    visible: bool,
    checked: bool
}

impl Visual {
    const DEF_LINE_H:u32 = 37;
    const DEF_FONT_SZ:u32 = 21;
    const DEF_FG:u32 = 0xFF880088;
    const DEF_STROKE:u32 = 0xFFFF00FF;
    const DEF_BG:u32 = 0xFF101010;
    const DEF_CHOICE_BG:u32 = 0xFF440044;
    const DEF_CHOICE_COLS:u32 = 7;
    const DEF_SEL_FG:u32 = 0xFF440044;
    const DEF_SEL_BG:u32 = 0xFFAAAAAA;
    fn set_content(&mut self, drw: x::Drawable, ctx:&CTX, style:&mut Style, mut value: &str) {
        if self.buf != x::Pixmap::none() { ctx.drop_pixmap(self.buf) }
        if self.mask != x::Pixmap::none() { ctx.drop_pixmap(self.mask) }
        if self.inv_mask != x::Pixmap::none() { ctx.drop_pixmap(self.inv_mask) }

        if self.tag.as_str()!="choices" {
            if value == ">>" {
                let ea = self.attrs["cnt-enum"].split("|");
                let mut le = "";
                let m = self.content.clone();
                for e in ea {
                    if le == "" {
                        self.content = e.to_string();
                    } else if le == m {
                        self.content = e.to_string();
                        break;
                    }
                    le = e;
                }
            } else { self.content = value.to_string() }
        }

        match self.tag.as_str() {
            "choices" => {
                if self.attrs.contains_key("items") { self.attrs.remove("items"); }
                println!("Choice items {:?}",value);
                self.attrs.insert("items".to_string(),value.to_string());
                let title = "Choose...:".to_string();
                let rv = title + value;
                let items = rv.split(":");
                let line_h = i16::from_str_radix(&style.prop_get("choices", "line-height", &format!("{}", Self::DEF_LINE_H)), 10).unwrap();
                let font_sz = i16::from_str_radix(&style.prop_get("choices", "font-size", &format!("{}", Self::DEF_FONT_SZ)), 10).unwrap();

                let sbg = format!("{:x}", Self::DEF_STROKE);
                let stroke = u32::from_str_radix(&style.prop_get("choices", "stroke", &sbg), 16).unwrap();

                let bgs = format!("{:x}", Self::DEF_SEL_BG);
                let fgs = format!("{:x}", Self::DEF_SEL_FG);
                //println!("Selected colors: {fgs}/{bgs}");

                let sbg = u32::from_str_radix(&style.prop_get(":selected", "bg", &bgs), 16).unwrap();
                let sfg = u32::from_str_radix(&style.prop_get(":selected","fg",&fgs),16).unwrap();

                let fnt = style.font_get(ctx,drw,"choices",self.fg,stroke,21).to_owned();
                let fnt_sel = style.font_get(ctx,drw,":selected",sfg,stroke,21).to_owned();
                self.buf = ctx.new_pixmap(drw,self.width,self.height);

                let bf = Drawable::Pixmap(self.buf);
                let gc = ctx.new_gc(bf,self.bg,self.fg);
                let gc_sel = ctx.new_gc(bf,sbg,sfg);
                ctx.rect(gc,bf,0,0,self.width,self.height);

                let blank = "".to_string();
                let sel = ":".to_string() + self.attrs.get("selected").unwrap_or(&blank).as_str() + ":";
                println!("Selected pattern {:?}",sel);

                let mut yc = 0;
                let mut xc = 16;
                let mut cw = self.width;

                let mask = ctx.new_mask(drw, self.width as i16, self.height as i16);

                for l in items {
              //      println!("Rendering choice: {l}");
                    let mut issel = false;
                    if sel!="::" { issel = sel.contains(&(":".to_string() + l + ":")) }

                    let (mut sw,sh) = fnt.measure_row(l, cw);
                    let pady = (line_h-sh as i16)/2 - line_h/8;
                    let mut padx = (cw as i16 - sw as i16)/2;
                    if issel {
                        ctx.rect(gc_sel,bf,xc,yc,cw,line_h as u16);
                        fnt.mask(ctx, mask, l, xc + padx, yc+pady, false, cw, line_h as u16);
                        let fgc = ctx.new_masked_gc(bf,mask,self.fg,self.bg);
                        fnt_sel.row(fgc, bf, ctx, self.buf, l, xc + padx, yc + pady, cw, line_h as u16);
                    } else {
                        if cw == self.width { padx = 0 };
                        fnt.mask(ctx, mask, l, xc + padx, yc+pady, false, cw, line_h as u16);
                        let fgc = ctx.new_masked_gc(bf,mask,self.fg,self.bg);
                        fnt.row(fgc, bf, ctx, self.buf, l, xc + padx, yc + pady, cw, line_h as u16);
                    }

                    xc += cw as i16;
                    if (xc+cw as i16) > self.width as i16 {
                        cw = self.width/Self::DEF_CHOICE_COLS as u16;;
                        xc = 0;
                        yc += line_h;
                    }
                }
                ctx.drop_pixmap(mask);
            }
            "lbl" => {
                if self.content.len() > 50 { self.content = self.content[0..50].to_string() }
                let sbg = format!("{:x}", Self::DEF_STROKE);
                let stroke = u32::from_str_radix(&style.prop_get(":stroke", "fg", &sbg), 16).unwrap();

                let mut pad:i16 = -1;
                let line_h = 64;
                let fnt = style.font_get(ctx,drw,"lbl", self.fg, stroke,31);
                let (mut sw,sh) = fnt.measure_row(&self.content,self.width);
                let yo = (line_h-sh as i16)/2;
                if pad == -1 { pad = yo; }
                sw += 2*pad as u16;
                self.width = sw;
                self.height = line_h as u16;
                self.buf = ctx.new_pixmap(drw,self.width,self.height);
                let bf = Drawable::Pixmap(self.buf);
                let gc = ctx.new_gc(bf,self.bg,self.fg);
                ctx.rect(gc,bf,0,0, self.width, self.height);
                fnt.row(gc,drw,ctx,self.buf,&self.content,pad,yo,self.width,self.height);
                self.mask = ctx.new_mask(drw,self.width as i16, self.height as i16);
                fnt.mask(ctx,self.mask,&self.content,pad,yo,false,self.width, self.height);
                self.inv_mask = ctx.new_mask(drw,self.width as i16, self.height as i16);
                fnt.mask(ctx,self.inv_mask,&self.content,pad,yo,true,self.width, self.height);
            }
            "i"=> {
                let sbg = format!("{:x}", Self::DEF_STROKE);
                let stroke = u32::from_str_radix(&style.prop_get(":stroke", "fg", &sbg), 16).unwrap();
                self.buf = ctx.img_from_alpha(drw,&self.content,8,self.width as i16, self.height as i16,stroke,self.fg);
                self.mask = ctx.mask_from_file(drw,&self.content,8, false, self.width as i16, self.height as i16);
                self.inv_mask = ctx.mask_from_file(drw,&self.content,8, true, self.width as i16, self.height as i16);
            }
            _ => {}
        }
        for a in self.attrs.clone().iter() {
            let aa = a.1.split(".").into_iter().collect::<Vec<&str>>();

            match a.0.as_str() {
                "fg" => {
                    self.fg = u32::from_str_radix(&a.1, 16).unwrap();
                }
                "bg" => {
                    self.bg = u32::from_str_radix(&a.1, 16).unwrap();
                    ctx.bg(self.window,self.bg);
                }
                "w" => {
                    self.width = Self::calc(&a.1,self.pwidth,self.pheight);
                }
                "h" => {
                    self.height = Self::calc(&a.1,self.pwidth,self.pheight);
                }
                "l" => {
                    if aa.len()>1 {
                        if aa[0]=="l" {
                            self.x = Self::anchor(&aa[1], self.lwidth + self.lx as u16);
                        } else {
                            self.x = Self::anchor(&aa[1], self.pwidth);
                        }
                    } else {
                        self.x = Self::calc(&a.1,self.pwidth,self.pheight) as i16;
                    }
                }
                "c" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1], self.pheight) - self.height as i16/2;;
                    } else {
                        self.y = Self::calc(&a.1,self.pwidth,self.pheight) as i16 - self.height as i16/2;
                    }
                }
                "m" => {
                    if aa.len()>1 {
                        self.x = Self::anchor(&aa[1], self.pwidth) - self.width as i16/2;;
                    } else {
                        self.x = Self::calc(&a.1,self.pwidth,self.pheight) as i16 - self.width as i16/2;
                    }
                }
                "r" => {
                    if aa.len()>1 {
                        self.x = Self::anchor(&aa[1],self.pwidth) - self.width as i16;
                    } else {
                        self.x = Self::calc(&a.1,self.pwidth,self.pheight) as i16 - self.width as i16;
                    }
                }
                "t" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1],self.pheight);
                    } else {
                        self.y = Self::calc(&a.1,self.pwidth,self.pheight) as i16;
                    }
                }
                "b" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1],self.pheight) - self.height as i16;
                    } else {
                        self.y = Self::calc(&a.1,self.pwidth,self.pheight) as i16 - self.height as i16;
                    }
                }
                _ => {}
            }
        }

        self.ax = self.iax + self.x;
        self.ay = self.iay + self.y;

        match self.tag.as_str() {
            "media"=>{
                println!("New media-buf {}x{}",self.width,self.height);
                self.buf = ctx.new_pixmap(drw,self.width,self.height);
            }
            "i"|"lbl"|"choices" => {}
            _ => {
                let fs = self.clone();
                let mut l = &fs;
                for c in self.children.iter_mut() {
                    c.anchor_fit_to(drw,ctx,style,l,&fs,self.ax,self.ay);
                    l = c;
                }
            }
        }

        ctx.pos(self.window,self.x,self.y);
        ctx.size(self.window,self.width,self.height);
    }
    fn calc(def:&str,mw:u16,mh:u16)->u16 {
        let l = def.len();
        let u = &def[l-2..l];
        let s:u32 = str::parse::<u32>(&def[0..l-2]).unwrap();
        match u {
            "pw" => (s*mw as u32/100) as u16,
            "ph" => (s*mh as u32/100) as u16,
            _ => s as u16
        }
    }
    fn anchor(def:&str,max:u16)->i16 {
        match def {
            "s"=> 0,
            "m"|"c"=> max as i16/2,
            _ => max as i16
        }
    }
    pub fn get_mut(&mut self, sel:&str)->Option<&mut Visual> {
        if self.tag==sel { return Option::from(self); }
        let ida = self.attrs.get("id");

        if ida.is_some() {
            if "#".to_string() + ida.unwrap().as_str()==sel { return Option::from(self); }
        }

        for c in self.children.iter_mut() {
            let r = c.get_mut(sel);
            if r.is_some() { return r; }
        }

        Option::None
    }
    pub fn select(&self, sel:&str)->Vec<&Visual> {
        let mut ret = vec![];

        if self.tag==sel { ret.push(self); }

        for c in self.children.iter() {
            ret.extend(c.select(sel));
        }

        ret
    }

    pub fn find<F: Fn(&mut Visual)->bool>(&mut self, f:&F)->Option<&mut Self> {
        if f(self) { return Option::from(self); }
        for c in self.children.iter_mut() {
            let ret = c.find(f);
            if ret.is_some () { return ret; }
        }
        None
    }
    
    pub fn new(ctx:&CTX,pwin:x::Window,n:&SceneNode)->Self {
        let mut visible = true;
        if n.attrs.contains_key("visible") { visible = bool::from_str(&n.attrs["visible"]).unwrap(); }

        let mut bg = Self::DEF_BG;
        if n.tag == "choices" { bg = Self::DEF_CHOICE_BG; }
        if n.attrs.contains_key("bg") { bg = u32::from_str_radix(&n.attrs["bg"],16).unwrap(); }
        let mut fg = Self::DEF_FG;
        if n.attrs.contains_key("fg") { fg = u32::from_str_radix(&n.attrs["fg"],16).unwrap(); }

        let mut window = pwin;
        match n.tag.as_str() {
            "root"=> {}
            "media"=> {
                window = ctx.new_basic_window(pwin, bg)
            }
            _ => {
                window = ctx.new_sub_window(pwin, bg)
            }
        }

        Self {
            x: 0,
            y: 0,
            iax: 0,
            iay: 0,
            ax: 0,
            ay: 0,
            width: 64,
            height: 64,
            lx: 0,
            ly: 0,
            lwidth: 64,
            lheight: 64,
            pwidth: 64,
            pheight: 64,
            inv_mask:x::Pixmap::none(),
            mask: x::Pixmap::none(),
            buf: x::Pixmap::none(),
            window,
            bg,
            fg,
            attrs: n.attrs.clone(),
            children: vec![],
            tag: n.tag.clone(),
            content: n.content.clone(),
            visible,
            checked: false
        }
    }

    pub fn show(&self,ctx:&Xcb) {
        if self.visible { ctx.show(self.window) }
        for c in &self.children {
            c.show(ctx);
        }
    }

    pub fn hide(&self,ctx:&Xcb) {
        ctx.hide(self.window);
        for c in &self.children {
            c.hide(ctx);
        }
    }

    pub fn demolish(&self,ctx:&Xcb) {
        for c in &self.children {
            c.demolish(ctx);
        }
        if self.tag!="root" {
            if self.window != x::Window::none() { ctx.drop_window(self.window); }
            if self.buf != x::Pixmap::none() { ctx.drop_pixmap(self.buf); }
            if self.mask != x::Pixmap::none() { ctx.drop_pixmap(self.mask); }
            if self.inv_mask != x::Pixmap::none() { ctx.drop_pixmap(self.inv_mask); }
        }
    }

    pub fn anchor_fit_to(&mut self,drw:x::Drawable,ctx:&CTX, style:&mut Style, l:&Visual,p:&Visual,ax:i16,ay:i16) {
        self.x = 0;
        self.y = 0;
        self.iax = ax;
        self.iay = ay;
        self.lx = l.x;
        self.ly = l.y;
        self.lwidth = l.width;
        self.lheight = l.height;
        self.pwidth = p.width;
        self.pheight = p.height;

        self.set_content(drw,ctx,style,&self.content.clone());
    }
}