use image::GenericImageView;

struct Nreq {

}

impl Nreq {
    fn new_mask(ctx:&Nxcb,file:&str)->x::Pixmap {
        let img = image::open(asset!(file,"png")).unwrap().to_rgba8();
        let width = img.width() as u16;
        let height = img.height() as u16;
        let pad = (32 - (width % 32))%32;
        let paddedw = width + pad;

        let pix:x::Pixmap = ctx.new_id::<x::Pixmap>();
        ctx.request(&x::CreatePixmap{
            depth: 1,
            pid: pix,
            drawable: ctx.drawable,
            width:paddedw,
            height
        });

        let mut pi = 0;
        let mut b: Vec<bool> = vec![];
        let mut u: Vec<u8> = vec![];
     //   let mut pixels = img.pi ;
        let allp : i16 = (height*paddedw) as i16;
        let mut tb:u8 = 0;
        let mut ix: u32 = 0;
        let mut iy:u32 = 0;
        for i in 0..allp {
            if ((i-8) % 8) == 0 {
                u.push(tb);
                tb = 0;
            }
            ix+=1;
            if ix == (paddedw as u32) { ix = 0; iy+=1 }
            if ix<(width as u32) && iy<(height as u32) && img.get_pixel(ix,iy).0[3] >127 { tb = tb | 1<<(i%8) as u8 };
        }
        ctx.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: 1,
            drawable: Drawable::Pixmap(pix),
            gc: Nreq::new_gc(ctx,Drawable::Pixmap(pix)),
            width: paddedw,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            data: &u.as_ref(),
        });
        pix
    }
    fn new_img(ctx:&Nxcb,file:&str)->x::Pixmap {
        let img = image::open(asset!(file,"png")).unwrap().to_rgba8();
        let width = img.width() as u16;
        let height = img.height() as u16;

        let pix:x::Pixmap = ctx.new_id::<x::Pixmap>();
        ctx.request(&x::CreatePixmap{
            depth: 24,
            pid: pix,
            drawable: ctx.drawable,
            width,
            height
        });

        ctx.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: ctx.depth,
            drawable: Drawable::Pixmap(pix),
            gc: Nreq::new_gc(ctx,Drawable::Pixmap(pix)),
            width,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        pix
    }
    fn new_img_backgrounded(ctx:&Nxcb,file:&str,bg:u32)->x::Pixmap {
        let mut img = image::open(asset!(file,"png")).unwrap().to_rgba8();
        let width = img.width() as u16;
        let height = img.height() as u16;

        let pix:x::Pixmap = ctx.new_id::<x::Pixmap>();
        ctx.request(&x::CreatePixmap{
            depth: 24,
            pid: pix,
            drawable: ctx.drawable,
            width,
            height
        });

        let bgr = ((bg >> 16) & 0xff) as f32;
        let bgg = ((bg >> 8) & 0xff) as f32;
        let bgb = (bg & 0xff) as f32;
        let u32: Vec<u32> = vec![];
        for mut p in img.pixels_mut() {
            let l = (p.0[3] as f32/255.0);
            let il = 1.0-l;
            p.0[0] = (bgb*il+p.0[0] as f32*l) as u8;
            p.0[1] = (bgg*il+p.0[1] as f32*l) as u8;
            p.0[2] = (bgr*il+p.0[2] as f32*l) as u8;
        }

        ctx.request(&x::PutImage{
            format: ImageFormat::ZPixmap,
            depth: ctx.depth,
            drawable: Drawable::Pixmap(pix),
            gc: Nreq::new_gc(ctx,Drawable::Pixmap(pix)),
            width,
            height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            data: &img.as_bytes(),
        });
        pix
    }
    fn new_pixmap(ctx:&Nxcb,width:u16,height:u16)->x::Pixmap {
        let pix:x::Pixmap = ctx.new_id::<x::Pixmap>();
        ctx.request(&x::CreatePixmap{
            depth: ctx.depth,
            pid: pix,
            drawable: ctx.drawable,
            width,
            height
        });
        pix
    }
    fn new_window(ctx:&mut Nxcb,bg:u32)->x::Window {
        let window:x::Window = ctx.new_id::<x::Window>();
        ctx.request(&x::CreateWindow{
            depth: ctx.depth,
            wid: window,
            parent: ctx.root,
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: ctx.visual_id,
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::EXPOSURE | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        ctx.drawable = Drawable::Window(window);
        ctx.gc = Nreq::new_gc(&ctx,ctx.drawable);
        window
    }
    fn new_sheer_window(ctx:&Nxcb,parent:x::Window,bg:u32)->x::Window {
        let window:x::Window = ctx.new_id::<x::Window>();
        ctx.request(&x::CreateWindow{
            depth: ctx.depth,
            wid: window,
            parent,
            x: 0,
            y: 0,
            width: 96,
            height: 96,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: ctx.visual_id,
            value_list: &[x::Cw::BackingPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::EXPOSURE | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        window
    }
    fn new_sub_window(ctx:&Nxcb,parent:x::Window,bg:u32)->x::Window {
        let window:x::Window = ctx.new_id::<x::Window>();
        ctx.request(&x::CreateWindow{
            depth: ctx.depth,
            wid: window,
            parent,
            x: 0,
            y: 0,
            width: 96,
            height: 96,
            border_width: 0,
            class: x::WindowClass::CopyFromParent,
            visual: ctx.visual_id,
            value_list: &[x::Cw::BackPixel(bg),x::Cw::EventMask(x::EventMask::OWNER_GRAB_BUTTON | x::EventMask::EXPOSURE | x::EventMask::POINTER_MOTION | x::EventMask::KEY_PRESS | x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE)],
        });
        window
    }
    fn new_gc(ctx:&Nxcb,d:Drawable) ->x::Gcontext{
        let oid = ctx.new_id();
        ctx.request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::Foreground(0xFFEEEEEE), Gc::Background(0xFF111111)]
        });
        oid
    }
    fn new_masked_gc(ctx:&Nxcb,d:Drawable,msk:x::Pixmap) ->x::Gcontext{
        let oid = ctx.new_id();
        ctx.dbg_request(&x::CreateGc {
            cid: oid,
            drawable: d,
            value_list: &[Gc::ClipMask(msk)]
        });
        oid
    }
  /*  fn opacity(ctx:&Nxcb,window:x::Window) {
        let data :u32 = 0xFFFFFFFF/2;
        //       double alpha = 0.8;
        //     unsigned long opacity = (unsigned long)(0xFFFFFFFFul * alpha);
        //   Atom XA_NET_WM_WINDOW_OPACITY = XInternAtom(display, "_NET_WM_WINDOW_OPACITY", False);
        ctx.request(&x::ChangeProperty{
            mode: x::PropMode::Replace,
            window,
            property: ctx.atoms.wm_opacity,
            r#type: x::ATOM_CARDINAL,
            data: &data.to_be_bytes().as_ref()
        });

        // XChangeProperty(display, win, XA_NET_WM_WINDOW_OPACITY, XA_CARDINAL, 32,
        //                PropModeReplace, (unsigned char *)&opacity, 1L);
    }*/

}