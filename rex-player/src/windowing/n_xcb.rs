use xcb::{x,Connection,Request,Xid};
use xcb::x::{Drawable, Gc, Gcontext};

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

impl Nxcb {

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
            drawable: Drawable::None,
            //ev_x:0,ev_y:0,
            //ev_width:0,
            //ev_height:0,
            //ev_window: x::Window::none(),
            //build_f: vec![],
            //built: HashMap::new(),
            // built_r: HashMap::new(),
           // app_w: x::Window::none(),
         //   icon_cache: HashMap::new()
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
    pub fn size(&self,window:x::Window,width:u16,height:u16) {
        self.request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::Width(width as u32),x::ConfigWindow::Height(height as u32)]
        });
    }
    pub fn bgc(&self,window:x::Window,bg:u32) {
       /* self.request(&x::ChangeProperty {
            mode: PropMode::Replace,
            window,
            property: (),
            r#type: (),
            data: &[],
        });*/
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