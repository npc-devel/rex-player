use std::ptr::null;

#[derive(Clone,Debug)]
struct SceneNode {
    tag: String,
    content: String,
    children: Vec<SceneNode>,
    attrs: strmap!(),
    id: u64
}

impl SceneNode {
    pub fn rebuild(&self, ctx:&CTX, win:x::Window,rv: &mut Visual) {
        for c in &self.children {
            c.build_in(ctx,win,rv);
        }
    }
    pub fn build_in(&self, ctx:&CTX, win:x::Window, p: &mut Visual) {
        let mut nwin = x::Window::none();
        if self.tag == "root" {
            for c in &self.children {
                c.build_in(ctx, win, p);
            }
        } else {
            let mut vis = Visual::new(ctx,win,self);
            for c in &self.children {
                c.build_in(ctx, vis.window, &mut vis);
            }
            p.children.push(vis);
        }
    }
    pub fn none()->Self {
        Self {
            tag: "".to_string(),
            content: "".to_string(),
            children: vec![],
            attrs: Default::default(),
            id: 0,
        }
    }
    pub fn new(v: &mut JsonValue) -> Self {
     //     println!("{:?}",v["content"]);
        let mut content="".to_string();
        let mut children:Vec<SceneNode> = vec![];
        if v["content"].is_array() {
            for k in v["content"].members() {
                children.push(SceneNode::new(&mut k.clone()));
            }
        } else {
            content = v["content"].to_string();
        }
        let ts = v["inner"].to_string().trim().to_string();
        let mut ta = ts.split(' ');
        let mut tag = ta.nth(0).unwrap();
        if tag == "null" {
            tag = "root";
        }
        let mut attrs:strmap!()=HashMap::new();
        let mut l :Option<&str>  = ta.nth(0);
        while l.is_some() {
            let mut te=l.unwrap().split("=");
            attrs.insert(te.nth(0).unwrap().to_string(),te.nth(0).unwrap().to_string());
            l = ta.nth(0);
        }
        Self {
            tag:tag.to_string(),
            children,
            content,
            attrs,
            id: random::<u64>()
        }
    }
}