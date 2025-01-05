use image::{DynamicImage, Rgba, ImageReader, EncodableLayout};
use xcb::x::ImageFormat;

struct Nsprite {
    pix: x::Pixmap,
    width: u16,
    height: u16
}

impl Nsprite {
    pub fn new(ctx:&Nxcb,file:&str)-> Self {
        let sf = asset!(file,"spr");
        let af = asset!(file,"png");
        let mf = asset!(file,"met");
        let img = image::open(af).unwrap().to_rgba8();
        let width = img.width() as u16;
        let height = img.height() as u16;
        let pix = Nreq::new_pixmap(ctx,width,height);
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




        Self {
            pix,width,height
        }
    }
    fn dump(&self,ctx:&Nxcb,win:x::Window) {
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