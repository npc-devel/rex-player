

struct Rhai {
    engine: Engine
}

impl Rhai {
    pub fn new()->Self {
        let mut engine = Engine::new();
        let fs = FilesystemPackage::new();
        fs.register_into_engine(&mut engine);
        engine.register_global_module(RandomPackage::new().as_shared_module());
        Self {
            engine
        }
    }

    pub fn exec(&self,mut script:&str)->String {
  //      println!("*********************************** \n{}\n *****************************", script);
        script = script.trim();
        if script.starts_with("??=") {
            let l = script.len();
            script = &script[3..l-2];
//            println!("*********************************** \n{}\n *****************************", script);
            self.engine.eval::<String>(&(script!("common","rhai") + "\n" + script)).unwrap().into()
        } else {
            "".into()
        }
    }
}