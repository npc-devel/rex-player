use xcb::x::{ChangeProperty, ConfigWindow, Property};

xcb::atoms_struct! {
    #[derive(Debug)]
    #[derive(Clone)]
    struct Atoms {
        wm_protocols    => b"WM_PROTOCOLS",
        wm_del_window   => b"WM_DELETE_WINDOW",
        wm_state   => b"_NET_WM_STATE",
        wm_fullscreen   => b"_NET_WM_STATE_FULLSCREEN",
        wm_opacity   => b"_NET_WM_WINDOW_OPACITY",
    }
}
struct Xcb {
    conn: Connection,
    atoms: Atoms,
    screen_n: i32,
    depth: u8,
    root: x::Window,
    visual_id: x::Visualid,
    gc: x::Gcontext,
    drawable: Drawable
}

struct XcbEvent {
    code: i32,
    window: x::Window,
    x:i16,y:i16,width:u16,height:u16
}
impl XcbEvent {
    pub const UNKNOWN:i32 = -1;
    pub const NONE:i32 = 0;
    pub const RENDER:i32 = 1;
    pub const B_DOWN:i32 = 2;
    pub const B_UP:i32 = 4;
    pub const RESIZE:i32 = 8;
    pub const MOTION:i32 = 16;
    pub fn new()->Self {
        Self {
            code: Self::NONE,
            window: x::Window::none(),
            x: 0,
            y: 0,
            width: 0,
            height: 0
        }
    }
}

impl Xcb {
    fn new_mask(&self,file:&str,pad:u16,inverted:bool,nw:i16,nh:i16)->x::Pixmap {
        let mut img = image::open(asset!(file,"png")).unwrap();
        let mut width = img.width() as u16;
        let mut height = img.height() as u16;
        let iw = width - 2 * pad;
        let ih = height - 2 * pad;
        //if nw > - 1  || nh > -1 {
        width = nw as u16;
        height = nh as u16;
        let iw = width - 2 * pad;
        let ih = height - 2 * pad;
        img = img.resize_to_fill(iw as u32, ih as u32, FilterType::Triangle);
        //}
        //}
        let img = img.to_rgba8();
        let bpad = (32 - (width % 32))%32;
        let paddedw = width + bpad;
        let ibpad = (32 - (iw % 32))%32;
        let ipaddedw = iw + ibpad;

        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: 1,
            pid: pix,
            drawable: self.drawable,
            width:paddedw,
            height
        });
  //      println!("{} {}",height,paddedw);
        let mut u: Vec<u8> = vec![];
        let ap : i64 = (ih as i64*ipaddedw as i64);
        let mut tb: u8 = 0;
        let mut ix: u32 = 0;
        let mut iy: u32 = 0;
        let xo: i32 = 0;
        for i in 0..ap {
            let sx = ix as i32 + xo;
            if sx>-1 && sx < (iw as i32) && iy < (ih as u32) {
                if  (!inverted && img.get_pixel(sx as u32,iy).0[3]>127) ||
                    (inverted && img.get_pixel(sx as u32,iy).0[3]<127) { tb = tb | 1<<(i%8) as u8 }
            } else {
                if inverted { tb = tb | 1<<(i%8) as u8 }
            }
            ix+=1;
            if ix>0 && (ix % 8) == 0 {
                u.push(tb);
                tb = 0;
            }
            if ix == ipaddedw as u32 { ix = 0; iy+=1 }
        }
        let mut fg = 0;
        let mut bg = 1;
        if inverted {
            fg = 1;
            bg = 0;
        }
        let gc = self.new_gc(Drawable::Pixmap(pix),fg,bg);
        let drawable = Drawable::Pixmap(pix);
        self.rect(gc,drawable,0,0,width,height);

        self.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: 1,
            drawable,
            gc,
            width: iw,
            height: ih,
            dst_x: pad as i16,
            dst_y: pad as i16,
            left_pad: 0,
            data: &u.as_ref(),
        });
        pix
    }
    fn new_img(&self,file:&str,pad:u16,nw:i16,nh:i16)->x::Pixmap {
        let mut img = image::open(asset!(file,"png")).unwrap();
        let mut width = img.width() as u16;
        let mut height = img.height() as u16;
        let iw = width - 2 * pad;
        let ih = height - 2 * pad;
        //if nw > - 1  || nh > -1 {
        width = nw as u16;
        height = nh as u16;
        let iw = width - 2 * pad;
        let ih = height - 2 * pad;
        img = img.resize_to_fill(iw as u32, ih as u32, FilterType::Triangle);
        //}
        //}
        let img = img.to_rgba8();

        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: 24,
            pid: pix,
            drawable: self.drawable,
            width,
            height
        });

        self.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: self.depth,
            drawable: Drawable::Pixmap(pix),
            gc: self.new_gc(Drawable::Pixmap(pix),1,0),
            width,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        pix
    }
    fn new_img_from_alpha(&self,file:&str,pad:u16,nw:i16,nh:i16,bg:u32,fg:u32)->x::Pixmap {
  //      println!("backgrounded {file}");
        let mut img = image::open(asset!(file,"png")).unwrap();
        let mut width = img.width() as u16;
        let mut height = img.height() as u16;
        let iw = width - 2 * pad;
        let ih = height - 2 * pad;
        //if nw > - 1  || nh > -1 {
            width = nw as u16;
            height = nh as u16;
            let iw = width - 2 * pad;
            let ih = height - 2 * pad;
            img = img.resize_to_fill(iw as u32, ih as u32, FilterType::Triangle);
        //}
        let mut img = img.to_rgba8();
        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: 24,
            pid: pix,
            drawable: self.drawable,
            width,
            height
        });

        let bgr = ((bg >> 16) & 0xff) as f32;
        let bgg = ((bg >> 8) & 0xff) as f32;
        let bgb = (bg & 0xff) as f32;
        let fgr = ((fg >> 16) & 0xff) as f32;
        let fgg = ((fg >> 8) & 0xff) as f32;
        let fgb = (fg & 0xff) as f32;

        let u32: Vec<u32> = vec![];
        for mut p in img.pixels_mut() {
            let l = (p.0[3] as f32/255.0);
            let il = 1.0-l;
            p.0[0] = (bgb*il+fgb*l) as u8;
            p.0[1] = (bgg*il+fgg*l) as u8;
            p.0[2] = (bgr*il+fgr*l) as u8;
        }

        self.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: self.depth,
            drawable: Drawable::Pixmap(pix),
            gc: self.new_gc(Drawable::Pixmap(pix),1,0),
            width:iw,
            height:ih,
            dst_x: pad as i16,
            dst_y: pad as i16,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        pix
    }
    fn new_pixmap(&self,width:u16,height:u16)->x::Pixmap {
        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: self.depth,
            pid: pix,
            drawable: self.drawable,
            width,
            height
        });
        pix
    }
    fn new_window(&self,bg:u32)->x::Window {
        let window:x::Window = self.new_id::<x::Window>();
        self.request(&x::CreateWindow{
            depth: self.depth,
            wid: window,
            parent: self.root,
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: self.visual_id,
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        self.select_input_cfg(window);
        window
    }
    fn new_buffered_window(&self,map:x::Pixmap)->x::Window {
        let window:x::Window = self.new_id::<x::Window>();
        self.request(&x::CreateWindow{
            depth: self.depth,
            wid: window,
            parent: self.root,
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: self.visual_id,
            value_list: &[x::Cw::BackPixmap(map),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        self.select_input_cfg(window);
        window
    }
    fn new_sheer_window(&self,parent:x::Window,mut bg:u32)->x::Window {
        let window:x::Window = self.new_id::<x::Window>();
        self.request(&x::CreateWindow{
            depth: self.depth,
            wid: window,
            parent,
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: self.visual_id,
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        window
    }
    fn new_sub_window(&self,parent:x::Window,bg:u32)->x::Window {
        let window:x::Window = self.new_id::<x::Window>();
        self.request(&x::CreateWindow{
            depth: self.depth,
            wid: window,
            parent,
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: self.visual_id,
            value_list: &[x::Cw::BackPixel(bg), x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        window
    }
    fn new_exposure_window(&self,parent:x::Window,bg:u32)->x::Window {
        let window:x::Window = self.new_id::<x::Window>();
        self.request(&x::CreateWindow{
            depth: self.depth,
            wid: window,
            parent,
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: self.visual_id,
            value_list: &[x::Cw::BackPixel(bg), x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::EXPOSURE | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        window
    }
    fn new_gc(&self,d:Drawable,fg:u32,bg:u32) ->x::Gcontext{
        let oid = self.new_id();
        self.request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::Foreground(fg), Gc::Background(bg)]
        });
        oid
    }
    fn new_masked_gc(&self,d:Drawable,msk:x::Pixmap,fg:u32,bg:u32) ->x::Gcontext{
        let oid = self.new_id();
        self.request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::Foreground(fg), Gc::Background(bg),Gc::ClipMask(msk)]
        });
        oid
    }
    pub fn wait_event(& self) -> XcbEvent {
        let mut ret = XcbEvent::new();
        loop {
            let eventr = self.conn.poll_for_event();
            if eventr.is_err() {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            let evento = eventr.unwrap();
            if evento.is_none() {
                return ret
            }
            let event = evento.unwrap();

            match event {
                Event::Present(xcb::present::Event::ConfigureNotify(event)) => {
                    ret.window = event.window();
                    ret.width = event.width();
                    ret.height = event.height();
                    ret.code = XcbEvent::RESIZE;

                }
                /*   xcb::Event::X(x::Event::MotionNotify(ev))=> {
                       let win = ev.event();
                       let bid = self.built.get(&win);
                       if bid.is_none() { (-1, x::Window::none(), 0, 0) }
                       else {
                           let mut parm:i64 = ev.event_x() as i64;
                           parm = parm << 32;
                           parm = parm | (ev.event_y() as i64);
                           //println!("{parm}");
                           (Nevent::MOTION, win, *bid.unwrap(), parm)
                       }
                   }*/
                X(Expose(event)) => {
                    ret.window = event.window();
                    ret.code = XcbEvent::RENDER;

                }/*
            X(Event::ButtonPress(event))=>{
                let win = event.event();
                let bid = self.built[&win];

                (Self::B_DOWN, win, bid, event.detail() as i64)
            }*/
                X(x::Event::ButtonRelease(event)) => {
                    ret.window = event.event();
                    ret.x = event.event_x();
                    ret.y = event.event_y();
                    ret.code = XcbEvent::B_UP;

                }
                _ => {
                    ret.code = XcbEvent::UNKNOWN;
                }
            }
            if ret.code != XcbEvent::UNKNOWN { break; }
        }
        ret
    }
    pub fn prepare(&mut self,window:x::Window) {
        self.drawable = Drawable::Window(window);
        self.gc = self.new_gc(self.drawable,0xFFFFFFFF,0xFF000000);
    }
    pub fn new()->Self {
        let (conn,screen_n) = xcb::Connection::connect_with_extensions(None, &[xcb::Extension::Present], &[]).unwrap();
        let setup = conn.get_setup();
        let screen = setup.roots().nth(0 as usize).unwrap();
        let visual_id = screen.root_visual();
        let root = screen.root();

        Self {
            depth: screen.root_depth(),
            atoms: Atoms::intern_all(&conn).expect("No atoms"),
            conn,
            screen_n,
            visual_id,
            root,
            gc: x::Gcontext::none(),
            drawable: Drawable::None
        }
    }
    fn new_id<T: xcb::Xid + xcb::XidNew>(&self)->T {
        self.conn.generate_id::<T>()
    }
    fn dbg_request(&self,req:&impl xcb::RequestWithoutReply) {
        let cookie = self.conn.send_request_checked(req);
        let result = self.conn.check_request(cookie);
        if result.is_err() {
            print!("dbg_request: {:?}\n\n", result.err())
        }
    }
    fn request(&self,req:&impl Request) {
        self.conn.send_request(req);
    }
    fn collect(&self) {
        self.conn.flush().unwrap();
    }

    pub fn hide(&self,window:x::Window) {
        self.request(&x::UnmapWindow {
            window
        });
    }
    pub fn show(&self,window:x::Window) {
        self.request(&x::MapWindow {
            window
        });
    }
    pub fn select_input_cfg(&self,window:x::Window) {
        let eid = self.new_id();
        self.request(&xcb::present::SelectInput {
            eid,
            window: window,
            event_mask: xcb::present::EventMask::CONFIGURE_NOTIFY
        });
    }
    pub fn pos(&self,window:x::Window,x:i16,y:i16) {
        self.request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::X(x as i32),x::ConfigWindow::Y(y as i32)]
        });
    }
    pub fn request_redraw(&self,win:x::Window,x:u16,y:u16,w:u16,h:u16) {
        let event = x::ExposeEvent::new(win,x,y,w,h,1);
        self.request(&x::SendEvent {
            propagate: false,
            destination: x::SendEventDest::Window(win),
            event_mask: x::EventMask::EXPOSURE,
            event: &event,
        });
    }
    pub fn size(&self,window:x::Window ,width:u16,height:u16) {
        self.request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::Width(width as u32),x::ConfigWindow::Height(height as u32)]
        });
    }

    pub fn bg(&self,window:x::Window,bg:u32) {
       self.request(&x::ChangeWindowAttributes {
           window,
           value_list: &[Cw::BackPixel(bg)]
       });
    }
    pub fn map_bg(&self,window:x::Window,map:x::Pixmap) {
        self.request(&x::ChangeWindowAttributes {
            window,
            value_list: &[Cw::BackPixmap(map) ]
        });
    }
    pub fn fill(&self,gc:Gcontext,drawable:Drawable,b:&[u8],dst_x:i16,dst_y:i16,width:u16,height:u16){
        self.request(&x::PutImage {
            format: x::ImageFormat::ZPixmap,
            drawable,
            gc,
            width,
            height,
            dst_x,
            dst_y,
            left_pad: 0,
            depth: self.depth,
            data: &b.as_ref()
        });
    }
    pub fn rect(&self,gc:Gcontext,drawable:Drawable,x:i16,y:i16,width:u16,height:u16) {
        let r = x::Rectangle {
            x,
            y,
            width,
            height,
        };

        self.request(&x::PolyFillRectangle {
            drawable,
            gc,
            rectangles: &[r],
        });
    }
    pub fn copy(&self,gc:Gcontext,src_drawable:Drawable,dst_drawable:Drawable,src_x:i16,src_y:i16,dst_x:i16,dst_y:i16,width:u16,height:u16) {
        self.request(&x::CopyArea {
            src_drawable,
            dst_drawable,
            gc,
            src_x,
            src_y,
            dst_x,
            dst_y,
            width,
            height
        });
    }
}