
struct Rhai {
    engine: Engine
}

impl Rhai {
    pub fn new()->Self {
        let mut engine = Engine::new();

        Self {
            engine
        }
    }

    pub fn exec(&self,script:&str)->String {
     //   println!("*********************************** \n{}\n *****************************", script);
        self.engine.eval::<String>(script).unwrap().into()
    }
}