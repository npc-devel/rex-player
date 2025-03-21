
#[macro_export]
macro_rules! asset {
    ($x:expr,$y:expr) => {
        {
            let temp_s = format!("{}/../assets/{}.{}",std::env::current_dir().unwrap().as_path().to_str().unwrap(),$x,$y);

            temp_s
        }
    }
}

#[macro_export]
macro_rules! script {
    ($x:expr,$y:expr) => {
        {
            let temp_s = format!("{}/src/scripts/{}.{}",std::env::current_dir().unwrap().as_path().to_str().unwrap(),$x,$y);

            std::fs::read_to_string(temp_s).unwrap()
        }
    }
}
#[macro_export]
macro_rules! blank {
    () => {
        {
            let temp_s = "".to_string();
  //          println!("{temp_s}");
            temp_s.clone()
        }
    }
}
#[macro_export]
macro_rules! view {
    ($x:expr,$y:expr) => {
        {
            let temp_s = format!("{}/src/views/{}.{}",std::env::current_dir().unwrap().as_path().to_str().unwrap(),$x,$y);
  //          println!("{temp_s}");
            std::fs::read_to_string(temp_s).unwrap()
        }
    }
}

#[macro_export]
macro_rules! style {
    ($x:expr) => {
        {
            let temp_s = format!("{}/src/styles/{}.{}",std::env::current_dir().unwrap().as_path().to_str().unwrap(),$x,"rhss");
  //          println!("{temp_s}");
            std::fs::read_to_string(temp_s).unwrap()
        }
    }
}

#[macro_export]
macro_rules! bsf {
    ($x:expr) => {
        {
            let lsp = &(std::env::home_dir().unwrap().to_str().unwrap().to_string() + "/rex-player");
            if !std::fs::exists(lsp).unwrap() {
                std::fs::create_dir_all(lsp).unwrap();
            }
            format!("{}/{}.{}",lsp,$x,"bsf")
        }
    }
}

#[macro_export]
macro_rules! intmap {
    ()=> { HashMap<String,i32> }
}

#[macro_export]
macro_rules! charmap {
    ()=> { HashMap<i32,HashMap<String,i32>> }
}

#[macro_export]
macro_rules! strmap {
    ()=> { HashMap<String,String> }
}

#[macro_export]
macro_rules! mapmap {
    ()=> { HashMap<String,HashMap<String,String>> }
}

#[macro_export]
macro_rules! vismap {
    ()=> { HashMap<u64,Visual> }
}
#[macro_export]
macro_rules! winmap {
    ()=> { HashMap<x::Window,u64> }
}
#[macro_export]
macro_rules! laymap {
    ()=> { HashMap<String,Layer> }
}
#[macro_export]
macro_rules! domlays {
    ()=> { Vec<(String,DomLayer)> }
}
#[macro_export]
macro_rules! resmap {
    ()=> { HashMap<u32,x::Pixmap> }
}

#[macro_export]
macro_rules! spritemap {
    ()=> { HashMap<String,Sprite> }
}

#[macro_export]
macro_rules! u {
    ($e:expr)=> { $e.unwrap() }
}

#[macro_export]
macro_rules! idvec {
    ()=> { Vec<u64> }
}

#[macro_export]
macro_rules! nmap {
    ()=> { HashMap::new() }
}

#[macro_export]
macro_rules! ensure {
    ($k:expr,$v:expr,$m:expr) => { 
        if $m.contains_key(&$k) { $m.remove(&$k); }
        $m.insert($k, $v); 
    }
}

#[macro_export]
macro_rules! clear {
    ($k:expr,$m:expr) => { 
        if $m.contains_key(&$k) { $m.remove(&$k); }         
    }
}