use xcb::{x, Connection, Event, Request, Xid};
use xcb::Event::X;
use xcb::x::{Cw, Drawable, Gc, Gcontext};
use xcb::x::Event::Expose;

include!("n_req.rs");
include!("n_sprite.rs");
include!("n_scene.rs");

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
struct Nxcb {
    conn: Connection,
    atoms: Atoms,
    screen_n: i32,
    depth: u8,
    root: x::Window,
    visual_id: x::Visualid,
    gc: x::Gcontext,
    drawable: Drawable
}

//impl Clone for Nxcb {
    /*fn clone(&self) -> Self {
        unsafe {
            Self {
                depth: self.depth,
                atoms: self.atoms.clone(),
                conn: xcb::Connection::from_raw_conn(self.conn.get_raw_conn()),
                screen_n: self.screen_n,
                visual_id: self.visual_id,
                root: self.root,
                gc: x::Gcontext::none(),
                drawable: Drawable::None
            }
        }
    }
}*/

struct Nevent {
    code: i32,
    window: x::Window,
    x:i16,y:i16,width:u16,height:u16
}
impl Nevent {
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

impl Nxcb {
    pub fn wait_event(& self) -> Nevent {
        let mut ret = Nevent::new();
        let evento = self.conn.poll_for_event().unwrap();
        if evento.is_none() {
            return ret
        }
        let event = evento.unwrap();/* {
            Err(xcb::Error::Connection(xcb::ConnError::Connection)) => {
                // graceful shutdown, likely "x" close button clicked in title bar
                panic!("unexpected error");            }
            Err(err) => {
                panic!("unexpected error: {:#?}", err);
            }
            Ok(event) => event,
        };*/
        match event {
            xcb::Event::Present(xcb::present::Event::ConfigureNotify(event))=> {
                ret.window = event.window();
                ret.width = event.width();
                ret.height = event.height();
                ret.code = Nevent::RESIZE;
                ret
            }
            /*xcb::Event::X(x::Event::MotionNotify(ev))=> {
                let win = ev.event();
                let bid = self.built.get(&win);
                if bid.is_none() { (-1, x::Window::none(), 0, 0) }
                else {
                    let mut parm:i64 = ev.event_x() as i64;
                    parm = parm << 32;
                    parm = parm | (ev.event_y() as i64);
                    //println!("{parm}");
                    (Self::MOTION, win, *bid.unwrap(), parm)
                }
            }*/
            X(Expose(event))=>{
                //   let win = event.window();
                ret.window = event.window();
                ret.code = Nevent::RENDER;
                ret
            }/*
            X(Event::ButtonPress(event))=>{
                let win = event.event();
                let bid = self.built[&win];

                (Self::B_DOWN, win, bid, event.detail() as i64)
            }*/
            X(x::Event::ButtonRelease(event))=>{
                ret.window = event.event();
                ret.x = event.event_x();
                ret.y = event.event_y();
                ret.code = Nevent::B_UP;
                ret
            }
            _ => {
                ret.code = Nevent::UNKNOWN;
                ret
            }
        }
    }
    pub fn prepare(&mut self) {
      //  self.gfx_ctx = Nreq::new_gc(self,Drawable::Window(self.root));
    //    self.drawable = Drawable::Window(self.root);
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
    pub fn size(&self,window:x::Window,width:u16,height:u16) {
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