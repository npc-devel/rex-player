use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use json::{array, JsonValue};
use rand::{random, thread_rng, Rng};
use xcb::x::Window;

struct Nnode {
    tag: String,
    content: String,
    children: Vec<Nnode>,
    attrs: strmap!(),
    id: u64
}
impl Nnode {
    pub fn select<F: Fn(&str,&Nnode,&Nnode,&mut idvec!())->()>(&self, sel:&str,parent:&Nnode, func: &F, results:&mut idvec!()) {
        func(sel,self,parent,results);
        for c in self.children.iter() {
            c.select(sel,self,func,results);
        }
    }
    pub fn traverse<F: Fn(&Nnode,&Nnode,&mut vismap!())->()>(&self, parent:&Nnode, func: &F, visuals:&mut vismap!()) {
        func(self,parent,visuals);
        for c in self.children.iter() {
            c.traverse(self,func,visuals);
        }
    }
    pub fn new(v: &mut JsonValue) -> Self {
     //   println!("{:?}",v["content"]);
        let mut content="".to_string();
        let mut children:Vec<Nnode> = vec![];
        if v["content"].is_array() {
            for k in v["content"].members() {
                children.push(Nnode::new(&mut k.clone()));
            }
        } else {
            content = v["content"].to_string();
        }
        let ts = v["tag"].to_string().trim().to_string();
        let mut ta = ts.split(' ');
        let tag = ta.nth(0).unwrap().to_string();
        let mut attrs:strmap!()=HashMap::new();
        let mut l :Option<&str>  = ta.nth(0);
        while l.is_some() {
            let mut te=l.unwrap().split("=");
            attrs.insert(te.nth(0).unwrap().to_string(),te.nth(0).unwrap().to_string());
            l = ta.nth(0);
        }
        Self {
            tag,
            children,
            content,
            attrs,
            id: random::<u64>()
        }
    }
    pub fn parse_inner(&mut self,inner:&str) {

    }
}
#[derive(Clone)]
struct Nvisual {
    key:u64,
    x:i16,
    y:i16,
    width:u16,
    height:u16,
    window: x::Window,
    bg: u32
}
struct Nscene {
    root: Nnode
}

impl Nscene {
    fn calc(def:&str,mw:u16,mh:u16)->u16 {
        let l = def.len();
        let u = &def[l-2..l];
        let s = str::parse::<u16>(&def[0..l-2]).unwrap();
        match u {
            "pw" => mw,
            "ph" => mh,
            _ => s
        }
    }
    fn anchor(def:&str,max:u16)->i16 {
        match def {
            "s"=> 0,
            "c"=> max as i16/2,
            _ => max as i16
        }
    }
    pub fn anchor_fit_to(&mut self, ctx:&mut Nxcb, x:i16, y:i16, width:u16, height:u16,visuals:&mut vismap!()) {
        let v = visuals.get_mut(&self.root.id).unwrap();
        v.x = x;
        v.y = y;
        v.width = width;
        v.height = height;
        for c in &self.root.children {
            c.traverse(&self.root, &|n: &Nnode, p: & Nnode,visuals:&mut vismap!() | {
                let vp = visuals.get_mut(&p.id).unwrap().clone();
                let v = visuals.get_mut(&n.id).unwrap();

                for a in n.attrs.iter() {
                    let aa = a.1.split('.').into_iter().collect::<Vec<&str>>();
                    match a.0.as_str() {
                        "w" => {
                            v.width = Self::calc(&a.1,vp.width,vp.height)
                        }
                        "h" => {
                            v.height = Self::calc(&a.1,vp.width,vp.height)
                        }
                        "l" => {
                            v.x = Self::anchor(&aa[1],vp.width);
                        }
                        "r" => {
                            v.x = Self::anchor(&aa[1],vp.width) - v.width as i16;
                        }
                        "t" => {
                            v.y = Self::anchor(&aa[1],vp.height);
                        }
                        "b" => {
                            v.y = Self::anchor(&aa[1],vp.height) - v.height as i16;
                        }
                        _ => {}
                    }
                }
                ctx.pos(v.window,v.x,v.y);
                ctx.size(v.window,v.width,v.height);
              //  if n.tag == "media" {
                //    le/t w = v.window;
                    /*thread::spawn(|| {
                        let mut cs = Nxcb::new();
                        Lffmpeg::stream_file(ctx, Drawable::Window(w), "/home/ppc/Videos/Samples/50MB_1080P_THETESTDATA.COM_mp4_new.mp4").expect("Bad file");
                    });*/
               // }

            },visuals);
        }
    }
    pub fn select(&mut self, sel:&str)->idvec!(){
        let mut results: idvec!() = vec![];
        for c in &self.root.children {
            c.select(sel, &self.root, &|sel: &str, n: &Nnode, p: &Nnode, results: &mut idvec!()| {
                if n.tag==sel { results.push(n.id); }
            }, &mut results);
        }
        results
    }
    pub fn build_in(&mut self, ctx:&mut Nxcb, win:x::Window)->vismap!(){
        let mut visuals: vismap!() = HashMap::new();
        visuals.insert(self.root.id, Nvisual {
            key: self.root.id,
            x: 0,
            y: 0,
            width: 48,
            height: 48,
            window:x::Window::none(),
            bg: 0xFF222222
        });
        for c in &self.root.children {
            c.traverse(&self.root,&|n:&Nnode,p:& Nnode,visuals  | {
                let mut bg = 0xFF222255;
                if n.attrs.contains_key("bg") { bg = u32::from_str_radix(&n.attrs["bg"],16).unwrap(); }
                let window = Nreq::new_sub_window(ctx, win, bg);
                ctx.show(window);
                visuals.insert(n.id, Nvisual {
                    key: n.id,
                    x: 0,
                    y: 0,
                    width: 48,
                    height: 48,
                    window,
                    bg
                });
            },&mut visuals);
        }
        visuals
    }
    pub fn new(file:&str)-> Self {
        let raw = view!(file,"rhai");
        let processed = Self::process(&raw);
        let jdoc = "{ \"content\": [ ".to_string() + processed.as_str() + " ] }";
     //   println!("***********************************************************\n\n{jdoc}\n\n*********************************************************************************");
        let mut dom = json::parse(&jdoc).unwrap();
        let root = Nnode::new(&mut dom);
        //print!("nodes: {:?}", dnode);
        Self {
            root
        }
    }
    fn process(raw:&str)->String {
        let mut pro =  raw.replace("\n","").trim().to_string();
        while pro.find("  ").is_some() {
            pro = pro.replace("  "," ");
        }
        pro = pro.replace("> <","><").trim().to_string();

      //  println!("nows: {pro}");

        pro = pro.replace("</>","]] }");
        pro = pro.replace(">","]], \"content\": [[");
        pro = pro.replace("<","{ \"tag\": [[");
        //let pro = view!("common","rhai").to_string() + ";\nlet res = " + &pro[1..];

        pro = pro.replace("[[{","[{");
        pro = pro.replace("}]]","}]");
        pro = pro.replace("}{","},{");
        pro = pro.replace("[[","\"");
        pro = pro.replace("]]","\"");
        //pro
        pro
    }
}