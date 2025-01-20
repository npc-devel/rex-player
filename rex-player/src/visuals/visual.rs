#[derive(Clone)]
struct Visual {
    x:i16,
    y:i16,
    ax:i16,
    ay:i16,
    width:u16,
    height:u16,
    inv_mask: x::Pixmap,
    mask: x::Pixmap,
    buf: x::Pixmap,
    window: x::Window,
    bg: u32,
    fg: u32,
    children: Vec<Visual>,
    attrs: strmap!(),
    tag: String,
    content: String
}

impl Visual {
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

    pub fn select(&self, sel:&str)->Vec<&Visual> {
        let mut ret = vec![];
        for c in self.children.iter() {
            ret.extend(c.select(sel));
        }
        if self.tag==sel { ret.push(self); }
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
    
    pub fn new(window:x::Window,bg:u32,fg:u32,n:&SceneNode)->Self {
        Self {
            x: 0,
            y: 0,
            ax: 0,
            ay: 0,
            width: 64,
            height: 64,
            inv_mask:x::Pixmap::none(),
            mask: x::Pixmap::none(),
            buf: x::Pixmap::none(),
            window,
            bg,
            fg,
            attrs: n.attrs.clone(),
            children: vec![],
            tag: n.tag.clone(),
            content: n.content.clone()
        }
    }

    pub fn show(&self,ctx:&Xcb) {
        ctx.show(self.window);
        for c in &self.children {
            c.show(ctx);
        }
    }

    pub fn anchor_fit_to(&mut self, ctx:&Xcb, style:&Style, l:&Visual,p:&Visual,ax:i16,ay:i16) {
        self.x = 0;
        self.y = 0;
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
                    self.width = Self::calc(&a.1,p.width,p.height);
                }
                "h" => {
                    self.height = Self::calc(&a.1,p.width,p.height);
                }
                "l" => {
                    if aa.len()>1 {
                        if aa[0]=="l" {
                            self.x = Self::anchor(&aa[1], l.width + l.x as u16);
                        } else {
                            self.x = Self::anchor(&aa[1], p.width);
                        }
                    } else {
                        self.x = Self::calc(&a.1,p.width,p.height) as i16;
                    }
                }
                "c" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1], p.height) - self.height as i16/2;;
                    } else {
                        self.y = Self::calc(&a.1,p.width,p.height) as i16 - self.height as i16/2;
                    }
                }
                "m" => {
                    if aa.len()>1 {
                        self.x = Self::anchor(&aa[1], p.width) - self.width as i16/2;;
                    } else {
                        self.x = Self::calc(&a.1,p.width,p.height) as i16 - self.width as i16/2;
                    }
                }
                "r" => {
                    if aa.len()>1 {
                        self.x = Self::anchor(&aa[1],p.width) - self.width as i16;
                    } else {
                        self.x = Self::calc(&a.1,p.width,p.height) as i16 - self.width as i16;
                    }
                }
                "t" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1],p.height);
                    } else {
                        self.y = Self::calc(&a.1,p.width,p.height) as i16;
                    }
                }
                "b" => {
                    if aa.len()>1 {
                        self.y = Self::anchor(&aa[1],p.height) - self.height as i16;
                    } else {
                        self.y = Self::calc(&a.1,p.width,p.height) as i16 - self.height as i16;
                    }
                }
                _ => {}
            }
        }
        self.ax = ax + self.x;
        self.ay = ay + self.y;

        match self.tag.as_str() {
            "lbl" => {
                let mut pad:i16 = -1;
                let line_h = 64;
                let fnt = style.fonts.get("_").unwrap();
                let (mut sw,sh) = fnt.measure_row(&self.content,self.width, self.height);
                let yo = (line_h-sh as i16)/2;
                if pad == -1 { pad = yo; }
                sw += 2*pad as u16;
                self.width = sw;
                self.height = line_h as u16;
                
                self.buf = ctx.new_pixmap(self.width,self.height);
                fnt.row(ctx,self.buf,&self.content,pad,yo,self.width, self.height);
                self.mask = ctx.new_mask(self.width as i16, self.height as i16);
                fnt.mask(ctx,self.mask,&self.content,pad,yo,false,self.width, self.height);
                self.inv_mask = ctx.new_mask(self.width as i16, self.height as i16);
                fnt.mask(ctx,self.inv_mask,&self.content,pad,yo,true,self.width, self.height);
            }
            "i" => {
                self.mask = ctx.mask_from_file(&self.content,8, false, self.width as i16, self.height as i16);
                self.inv_mask = ctx.mask_from_file(&self.content,8, true, self.width as i16, self.height as i16);
                self.buf = ctx.img_from_alpha(&self.content,8,self.width as i16, self.height as i16,self.bg,self.fg);
            }
            "media" => {
             //   let drw = Drawable::Window(self.window.clone());
                let vwidth = self.width.clone();
                let vheight = self.height.clone();
                let map = ctx.new_pixmap(vwidth,vheight);
                self.buf = map;
            }
            _ => {
                let fs = self.clone();
                let mut l = &fs;
                for c in self.children.iter_mut() {
                    c.anchor_fit_to(ctx,style,l,&fs,self.ax,self.ay);
                    l = c;
                }
            }
        }

        ctx.pos(self.window,self.x,self.y);
        ctx.size(self.window,self.width,self.height);
    }
}