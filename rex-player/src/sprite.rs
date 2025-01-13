use std::io::Read;

struct Sprite {
    pix: x::Pixmap,
    width: u16,
    height: u16,
    map: charmap!()
}

impl Sprite {
    pub fn new(ctx:&Xcb,file:&str,fg:u32,bg:u32)-> Self {
        let ff = asset!(file,"fnt");
        let pf = asset!(file,"png");
        let mf = asset!(file,"met");
        let mut img = image::open(pf).unwrap().to_rgba8();

        let pfg:[f32;3] = [
            ((0x00FF0000 & fg) >> 16) as f32,
            ((0x0000FF00 & fg) >> 8) as f32,
            (0x000000FF & fg) as f32
        ];
        let pbg:[f32;3] = [
            ((0x00FF0000 & bg) >> 16) as f32,
            ((0x0000FF00 & bg) >> 8) as f32,
            (0x000000FF & bg) as f32
        ];

        for i in img.pixels_mut() {
            let l: f32 = (i[1] as f32 + i[2] as f32 + i[3] as f32)/765.0;
            let li = 1.0-l;
            i[2] = (l*pfg[0] + li*pbg[0]) as u8;
            i[1] = (l*pfg[1] + li*pbg[1]) as u8;
            i[0] = (l*pfg[2] + li*pbg[2]) as u8;
        }

        let width = img.width() as u16;
        let height = img.height() as u16;
        let pix = ctx.new_pixmap(width,height);
        ctx.request(&x::PutImage {
            format: ImageFormat::ZPixmap,
            drawable: Drawable::Pixmap(pix),
            gc: ctx.gc,
            width,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            depth: ctx.depth,
            data: &img.as_bytes(),
        });

        let fds = std::fs::read_to_string(ff).unwrap();
        let lines = fds.split("\n");
        let mut jdoc = "".to_string();
        for l in lines {
            if l.starts_with("char ") {
                let mut l = l.replace("char id=",",\"").replace(" x=","\":{\"x="); 
                l = l.replace(" ","\", \"").replace("=","\": \"") + "\"}";
                jdoc += &l;
            }
        }
        jdoc = "{ ".to_string() + &jdoc[1..] + " }";
        let mut dom = json::parse(&jdoc).unwrap();
        let mut map: charmap!() = nmap!();
        for m in dom.entries() {
            let mut vals : intmap!() = nmap!();
            for vp in m.1.entries() {
                vals.insert(vp.0.to_string(),i32::from_str_radix(vp.1.as_str().unwrap(),10).unwrap());
            } 
            map.insert(i32::from_str_radix(m.0,10).unwrap(),vals);
        }
       // println!("sprite {:?}->",map);
        Self {
            pix,
            width,
            height,
            map
        }
    }
    fn measure_row(&self,cnt:&str,pad:i16,w:u16,h:u16)->(u16,u16) {
        let mut x = pad;
        let mut y = pad;
        let mut mh = 0;
        for c in cnt.chars() {
            let key = c as i32;
            if self.map.contains_key(&key) {
                let info = self.map.get(&key).unwrap();
                let h = *info.get("height").unwrap() as u16;
                if h>mh { mh = h; }
                x += *info.get("xadvance").unwrap() as i16;
            }
        }
        ((x+pad) as u16,mh+2*pad as u16)
    }
    
    fn row(&self,ctx:&Xcb,buf:x::Pixmap,cnt:&str,pad:i16) {
        let mut x = pad;
        let mut y = pad;
        let mut mh = 0;
        for c in cnt.chars() {
            let key = c as i32;
            if self.map.contains_key(&key) {
                let info = self.map.get(&key).unwrap();
                let h = *info.get("height").unwrap() as u16;
                let ho = 10;
                if h>mh { mh = h; }
                ctx.dbg_request(&x::CopyArea {
                    src_drawable: x::Drawable::Pixmap(self.pix),
                    dst_drawable: x::Drawable::Pixmap(buf),
                    gc: ctx.gc,
                    src_x: *info.get("x").unwrap() as i16,
                    src_y: *info.get("y").unwrap() as i16,
                    dst_x: x + *info.get("xoffset").unwrap() as i16,
                    dst_y: y + *info.get("yoffset").unwrap() as i16 - ho as i16,
                    width: *info.get("width").unwrap() as u16,
                    height: h,
                });
                x += *info.get("xadvance").unwrap() as i16;
            }
        }
    }

    /*fn font_text(&self,win:Window, txt:&str, mut x:i16, y:i16,ico_indent:i32,mut ssf:i16) -> i16 {
        let mut indent = 0;
        let gc: x::Gcontext = self.connection.generate_id();
        self.connection.send_request(&x::CreateGc {
            cid: gc,
            drawable: x::Drawable::Window(win),
            value_list: &[]
        });
        let mut pix = &self.fmain;
        if ssf == 1 { pix = &self.fhdr; }
        let mut map = &self.fmap;
        let mut icon = false;
        for c in txt.bytes() {
            if c == '~' as u8 {
                icon = true;
                indent = ico_indent;
                pix = &self.fico;
                map = &self.fico_map;
                ssf = 2;
                x+=8;
                continue;
            }

            if c == '`' as u8 {
                if icon {
                    pix = &self.fico_caret;
                } else {
                    if pix == &self.fmain_caret {
                        pix = &self.fmain;
                        if ssf == 1 { pix = &self.fhdr; }
                    } else {
                        pix = &self.fmain_caret;
                        if ssf == 1 { pix = &self.fhdr_caret; }
                    }
                }
                continue;
            }

            //  println!("{}", c);
            let info = map.get(&(c as i32));
            if info.is_some() {
                let info = info.unwrap();

                self.connection.send_request(&x::CopyArea {
                    src_drawable: x::Drawable::Pixmap(*pix),
                    dst_drawable: x::Drawable::Window(win),
                    gc,
                    src_x: *info.get("x").unwrap() as i16 / ssf as i16,
                    src_y: *info.get("y").unwrap() as i16 / ssf as i16,
                    dst_x: x + *info.get("xoffset").unwrap() as i16 / ssf as i16,
                    dst_y: indent as i16 +y + *info.get("yoffset").unwrap() as i16 / ssf as i16,
                    width: *info.get("width").unwrap() as u16 / ssf as u16,
                    height: *info.get("height").unwrap() as u16 / ssf as u16,
                });
                x += *info.get("xadvance").unwrap() as i16 / ssf as i16;
            }
            if icon {
                icon = false;
                ssf = 1;
                pix = &self.fmain;
                if ssf == 1 { pix = &self.fhdr; }
                map = &self.fmap;
                indent = 0;
            }
        }
        x
    }*/
    
    fn dump(&self,ctx:&Xcb,win:x::Window) {
        ctx.request(&x::CopyArea {
            src_drawable: Drawable::Pixmap(self.pix),
            dst_drawable: Drawable::Window(win),
            gc: ctx.gc,
            src_x: 0,
            src_y: 0,
            dst_x: 0,
            dst_y: 0,
            width: self.width,
            height: self.height
        });
    }
}