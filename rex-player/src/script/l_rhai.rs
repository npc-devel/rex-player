use std::thread::spawn;
use rhai::{Engine, EvalAltResult};
struct Lrhai {
    engine: Engine
}

impl Lrhai {

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
   /* pub fn test() -> Result<(), Box<EvalAltResult>>
    {
        let

        let result = engine.eval::<i64>("40 + 2")?;
        //                      ^^^^^^^ required: cast the result to a type

        println!("Answer: {result}");             // prints 42

        Ok(())
    }*/
}