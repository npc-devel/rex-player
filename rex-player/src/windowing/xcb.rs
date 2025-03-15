use std::any::Any;
use xcb::{xfixes, BaseEvent};
use xcb::x::{ChangeProperty, ConfigWindow, Property, Screen};

xcb::atoms_struct! {
    #[derive(Debug)]
    #[derive(Clone)]
    struct Atoms {
        wm_title       => b"TITLE",
        wm_cursor       => b"CURSOR",
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
    screen: x::ScreenBuf,
    screen_n: i32,
    depth: u8,
    root: x::Window,
    visual_id: x::Visualid,
    s_width: u16,
    s_height: u16,
    master_window: x::Window,
    blank_cursor: x::Cursor
}

#[derive(Debug)]
struct XcbEvent {
    code: i32,
    window: Window,
    button: x::Button,
    x:i16,
    y:i16,
    width:u16,
    height:u16
}

impl XcbEvent {
    pub const UNKNOWN:i32 = -1;
    pub const NONE:i32 = 0;
    pub const RENDER:i32 = 1;
    pub const B_DOWN:i32 = 2;
    pub const B_UP:i32 = 4;
    pub const RESIZE:i32 = 8;
    pub const MOTION:i32 = 16;
    pub const SCROLL_DOWN:i32 = 32;
    pub const SCROLL_UP:i32 = 64;
    pub const CLOSE:i32 = 128;
    pub fn new()->Self {
        Self {
            code: Self::NONE,
            window: x::Window::none(),
            button: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0
        }
    }
}

impl Xcb {
    fn new_mask(&self,drawable:x::Drawable,width:i16,height:i16)->x::Pixmap {
        let bpad = (32 - (width % 32))%32;
        let paddedw = width + bpad;

        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: 1,
            pid: pix,
            drawable,
            width: paddedw as u16,
            height: height as u16
        });
        pix
    }
    fn mask_from_file(&self,drawable:x::Drawable,file:&str,pad:u16,inverted:bool,nw:i16,nh:i16)->x::Pixmap {
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
            drawable,
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
        self.drop_gc(gc);
        pix
    }
    fn new_img(&self,drawable:x::Drawable,file:&str,pad:u16,nw:i16,nh:i16)->x::Pixmap {
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
            drawable,
            width,
            height
        });

        let igc = self.new_gc(Drawable::Pixmap(pix),1,0);
        self.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: self.depth,
            drawable: Drawable::Pixmap(pix),
            gc: igc,
            width,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        self.drop_gc(igc);
        pix
    }
    fn img_from_alpha(&self,drawable:x::Drawable,file:&str,pad:u16,nw:i16,nh:i16,bg:u32,fg:u32)->x::Pixmap {
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
            drawable,
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

        let igc = self.new_gc(Drawable::Pixmap(pix),1,0);
        self.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: self.depth,
            drawable: Drawable::Pixmap(pix),
            gc: igc,
            width:iw,
            height:ih,
            dst_x: pad as i16,
            dst_y: pad as i16,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        self.drop_gc(igc);
        pix
    }

    fn drop_window(&self, window: x::Window) {
        self.request(&x::UnmapWindow { window });
    }

    fn drop_pixmap(&self, pixmap: x::Pixmap) {
        self.request(&x::FreePixmap { pixmap });
    }
    fn drop_gc(&self, gc: x::Gcontext) {
        self.request(&x::FreeGc { gc });
    }

    fn new_pixmap(&self,drawable:x::Drawable,width:u16,height:u16)->x::Pixmap {
        let pix:x::Pixmap = self.new_id::<x::Pixmap>();
        self.request(&x::CreatePixmap{
            depth: self.depth,
            pid: pix,
            drawable,
            width,
            height
        });
        pix
    }
    /*fn new_window(&self,bg:u32)->x::Window {
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
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE),x::Cw::Cursor(x::CURSOR_NONE)],
        });
        self.select_input_cfg(window);
        window
    }*/
   /* fn new_buffered_window(&self,map:x::Pixmap)->x::Window {
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
            value_list: &[x::Cw::BackPixmap(map),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE),x::Cw::Cursor(x::CURSOR_NONE)],
        });
        self.select_input_cfg(window);
        window
    }*/
    /*fn new_sheer_window(&self,parent:x::Window,mut bg:u32)->x::Window {
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
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE),x::Cw::Cursor(x::CURSOR_NONE)],
        });
        window
    }*/
    
    fn new_gl_window(&self,parent:x::Window,bg:u32)->x::Window {
        let cmap: x::Colormap = self.new_id::<x::Colormap>();
        let win: x::Window = self.new_id::<x::Window>();

        let fbc = get_glxfbconfig(
            self.conn.get_raw_dpy(),
            self.screen_n,
            &[
                GLX_X_RENDERABLE,
                1,
                GLX_DRAWABLE_TYPE,
                GLX_WINDOW_BIT,
                GLX_RENDER_TYPE,
                GLX_RGBA_BIT,
                GLX_X_VISUAL_TYPE,
                GLX_TRUE_COLOR,
                GLX_RED_SIZE,
                8,
                GLX_GREEN_SIZE,
                8,
                GLX_BLUE_SIZE,
                8,
                GLX_ALPHA_SIZE,
                8,
                GLX_DEPTH_SIZE,
                24,/*
                GLX_STENCIL_SIZE,
                8,
                GLX_DOUBLEBUFFER,
                1,*/
                0,
            ],
        );

        let vi_ptr: *mut xlib::XVisualInfo =
            unsafe { glXGetVisualFromFBConfig(self.conn.get_raw_dpy(), fbc) };
        let vi = unsafe { *vi_ptr };

        self.request(&x::CreateColormap {
            alloc: x::ColormapAlloc::None,
            mid: cmap,
            window: self.screen.root(),
            visual: vi.visualid as u32,
        });

        self.request(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: win,
            parent,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            border_width: 0,
            class: x::WindowClass::InputOutput,
            visual: vi.visualid as u32,
            value_list: &[
                x::Cw::BackPixel(self.screen.white_pixel()),
                x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS),
                x::Cw::Colormap(cmap),
            ],
        });

        unsafe {
            xlib::XFree(vi_ptr as *mut c_void);
        }
        
        win
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
            value_list: &[x::Cw::BackPixel(bg), x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE),x::Cw::Cursor(x::CURSOR_NONE)],
        });
        window
    }
   /* fn new_basic_window(&self,parent:x::Window,bg:u32)->x::Window {
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
            value_list: &[x::Cw::BackPixel(bg), x::Cw::EventMask(x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE),x::Cw::Cursor(x::CURSOR_NONE)],
        });
        window
    }*/
    //fn hide_cursor(&self,window: x::Window) {
       // println!("Hide cursor");
     //   self.request(&xfixes::HideCursor { window });
    //}
    //fn show_cursor(&self,window: x::Window) {
        //println!("Show cursor");
        //self.request(&xfixes::ChangeCursor { source: /x::ATOM_CURSOR, destination: () });
    //}
   /* fn new_exposure_window(&self,parent:x::Window,bg:u32)->x::Window {
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
    }*/
    fn new_gc(&self,d:Drawable,fg:u32,bg:u32) ->x::Gcontext{
        let oid = self.new_id();
        self.request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::Foreground(fg), Gc::Background(bg),Gc::GraphicsExposures(false)]
        });

        //self.request( )
        oid
    }
    fn new_masked_gc(&self,d:Drawable,msk:x::Pixmap,fg:u32,bg:u32) ->x::Gcontext{
        let oid = self.new_id();
        self.request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::Foreground(fg), Gc::Background(bg),Gc::GraphicsExposures(false),Gc::ClipMask(msk)]
        });
        oid
    }
    pub fn wait_event(& self) -> XcbEvent {
        let mut ret = XcbEvent::new();
        let mut idx = 1;
        loop {
            idx -= 1;
            if idx < 0 { //print!("f");
                break;
            }
         //   print!("p");
            let eventr = self.conn.poll_for_event();
            if eventr.is_err() {
            //    print!("e");
                return ret;
            }
            let evento = eventr.unwrap();
            if evento.is_none() {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }

            ret.code = XcbEvent::NONE;
            let event = evento.unwrap();
            //print!(".{:?}.",event);
            match event {
                X(x::Event::ClientMessage(ev)) => {
                    if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                        if atom == self.atoms.wm_del_window.resource_id() {
                            ret.code = XcbEvent::CLOSE;
                        }
                    }
                }
                Event::Present(xcb::present::Event::ConfigureNotify(event)) => {
                    ret.window = event.window();
                    ret.width = event.width();
                    ret.height = event.height();
                    ret.code = XcbEvent::RESIZE;
                }
                X(x::Event::MotionNotify(event))=> {
                    //println!("Motion event: {:?}", event);
                    ret.window = event.child();
                    if ret.window == x::Window::none() { ret.window = event.event() }
                    ret.x = event.event_x();
                    ret.y = event.event_y();
                    ret.code = XcbEvent::MOTION;
                }
                X(Expose(event)) => {
                    //i//f event.count()==0 {
                        ret.window = event.window();
                        ret.code = XcbEvent::RENDER;
                   // }
                }
                X(x::Event::ButtonPress(event))=>{
                    let btn = event.detail();
                    match btn {
                        4=> { ret.code = XcbEvent::SCROLL_UP; }
                        5=> { ret.code = XcbEvent::SCROLL_DOWN; }
                        _=>{ ret.code = XcbEvent::B_DOWN; }
                    }
                    ret.window = event.event();
                    ret.x = event.event_x();
                    ret.y = event.event_y();
                    ret.button = btn;
                }
                X(x::Event::ButtonRelease(event))=>{
                    let btn = event.detail();
                    match btn {
                        4|5=> { continue }
                        _=>{ ret.code = XcbEvent::B_UP; }
                    }

                    ret.window = event.event();
                    ret.x = event.event_x();
                    ret.y = event.event_y();
                    ret.button = btn;
                }
                _ => {
                //    println!("{:?}",event);
                    ret.code = XcbEvent::UNKNOWN;
                }
            }

            //print!(".{:?}.",ret.code);
            if ret.code != XcbEvent::NONE && ret.code != XcbEvent::UNKNOWN  {
             //   print!("XcbEvent: {:?}",ret);
                break;
            }

        }
        ret
    }
    /*pub fn prepare(&mut self,window:x::Window) {
        self.drawable = Drawable::Window(window);
        self.gc = self.new_gc(self.drawable,0xFFFFFFFF,0xFF000000);
    }*/
    pub fn new()->Self {
        let (conn, screen_n) =
            u!(xcb::Connection::connect_with_xlib_display_and_extensions(&[], &[xcb::Extension::Dri2,xcb::Extension::Present]));

        conn.set_event_queue_owner(xcb::EventQueueOwner::Xcb);

        let glx_ver = u!(conn.wait_for_reply(conn.send_request(&glx::QueryVersion {
            major_version: 1,
            minor_version: 3,
        })));
        assert!(glx_ver.major_version() >= 1 && glx_ver.minor_version() >= 3);

        let setup = conn.get_setup();
        let screen = setup.roots().nth(0 as usize).unwrap().to_owned();
        let visual_id = screen.root_visual();
        let root = screen.root();
        let s_width = screen.width_in_pixels();
        let s_height = screen.height_in_pixels();
        let depth = screen.root_depth().clone();

        let bg = 0xFF000000;
        let master_window = conn.generate_id();
        conn.send_request(&x::CreateWindow{
            depth,
            wid: master_window,
            parent: root,
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: visual_id,
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });

        let atm = Atoms::intern_all(&conn).expect("No atoms");

        let cookie = conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: master_window,
            property: x::ATOM_WM_NAME,
            r#type: x::ATOM_STRING,
            data: b"REX"
        });
        // And check for success again
        conn.check_request(cookie).expect("couldn't change property");

        let eid = conn.generate_id();
        conn.send_request(&xcb::present::SelectInput {
            eid,
            window: master_window,
            event_mask: xcb::present::EventMask::CONFIGURE_NOTIFY
        });

        conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: master_window,
            property: atm.wm_protocols,
            r#type: x::ATOM_ATOM,
            data: &[atm.wm_del_window],
        });

        let cid = conn.generate_id();
        let pix = conn.generate_id();
        conn.send_request(&x::CreatePixmap{
            depth: 1,
            pid: pix,
            drawable: Drawable::Window(master_window),
            width: 32,
            height: 32
        });
        
        conn.send_request(&x::CreateCursor {
            cid,
            source: pix,
            mask: pix,
            fore_red: 0,
            fore_green: 0,
            fore_blue: 0,
            back_red: 0,
            back_green: 0,
            back_blue: 0,
            x: 0,
            y: 0,
        });
        conn.send_request(&x::FreePixmap { pixmap: pix });

        Self {
            blank_cursor: cid,
            depth,
            screen,
            atoms: atm,
            conn,
            screen_n,
            visual_id,
            root,
            s_width,
            s_height,
            master_window
        }
    }
    fn new_id<T: xcb::Xid + xcb::XidNew>(&self)->T {
        self.conn.generate_id::<T>()
    }
    fn dbg_request(&self,req:&impl xcb::RequestWithoutReply) {
        let cookie = self.conn.send_request_checked(req);
        let result = self.conn.check_request(cookie);
        if result.is_err() {
            println!("Bad dbg_request: {:?}\n\n", result.err())
        }
    }
    fn request(&self,req:&impl Request) {
        self.conn.send_request(req);
    }
    fn collect(&self) {
        let c = self.conn.flush();
        /*if !c.is_ok() {
            let (conn,screen_n) = xcb::Connection::connect_with_extensions(None, &[xcb::Extension::Present], &[]).unwrap();
            self.conn = conn;
        }*/
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
    pub fn cursor_vis(&self,vis:bool) {
        if vis {
            let cursor_context = CursorContext::new(&self.conn, &self.screen).unwrap();
            let cur = cursor_context.load_cursor(Cursor::LeftPtr);
            self.request(&x::ChangeWindowAttributes {
                window: self.master_window,
                value_list: &[x::Cw::Cursor(cur)],
            });
        } else {
            self.request(&x::ChangeWindowAttributes {
                window: self.master_window,
                value_list: &[x::Cw::Cursor(self.blank_cursor)],
            });
        }
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