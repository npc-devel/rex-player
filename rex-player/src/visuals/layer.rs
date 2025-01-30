
include!("scene_node.rs");
include!("visual.rs");

#[derive(Clone, CustomType)]
struct Layer {
    root: SceneNode,
    root_visual: Visual
}
impl Layer {
    pub fn select(&self, sel:&str) ->Vec<&Visual> {
        self.root_visual.select(sel)
    }

    pub fn visual(&mut self, res:u32) ->Option<&mut Visual> {
        self.root_visual.find(&|n:& mut Visual|{
            n.window.resource_id() == res
        })
    }
    pub fn fit_all(&mut self, ctx:&Xcb, style:&Style, w:u16, h:u16) {
        self.root_visual.width = w;
        self.root_visual.height = h;
        let mut cs = self.root_visual.clone();
        let mut last = &cs;
        for c in self.root_visual.children.iter_mut() {
            c.anchor_fit_to(ctx,style,&last,&cs,0,0);
            last = c;
        }
    }
    pub fn build_all(&mut self, ctx:&Xcb, win:x::Window) {
        self.root_visual = Visual::new(win, 0xFF111111,0xFFFFFFFF,&self.root);
        for c in &self.root.children {
            c.build_in(ctx,win,&mut self.root_visual);
        }
    }
   pub fn new(file:&str,ctx:&Xcb,win:x::Window,bg:u32,fg:u32,w:u16,h:u16)->Self {
        let raw = view!(file,"rhai");
        let processed = Self::process(&raw);
        let jdoc = "{ \"content\": [ ".to_string() + processed.as_str() + " ] }";
//        println!("***********************************************************\n\n{jdoc}\n\n*********************************************************************************");
        let mut dom = json::parse(&jdoc).unwrap();
        let root = SceneNode::new(&mut dom);
        let mut root_visual = Visual::new(win, bg, fg, &root);

        for c in &root.children {
           c.build_in(ctx,win,&mut root_visual);
        }

        Self {
           root_visual,
           root
        }
    }

    fn process(raw:&str)->String {
        let mut pro =  raw.replace("\n","").trim().to_string();
        while pro.find("  ").is_some() {
            pro = pro.replace("  "," ");
        }
        pro = pro.replace("\"","\\\"");
        pro = pro.replace("> <","><");
        pro = pro.replace("</>","]] }");
        pro = pro.replace(">","]], \"content\": [[");
        pro = pro.replace("<","{ \"inner\": [[");

        pro = pro.replace("[[{","[{");
        pro = pro.replace("}]]","}]");
        pro = pro.replace("}{","},{");
        pro = pro.replace("[[","\"");
        pro = pro.replace("]]","\"");
     //   println!("{pro}");
        pro
    }
}


