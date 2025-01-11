
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
macro_rules! strmap {
    ()=> { HashMap<String,String> }
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
macro_rules! resmap {
    ()=> { HashMap<u32,x::Pixmap> }
}

#[macro_export]
macro_rules! u {
    ()=> { unwrap() }
}

#[macro_export]
macro_rules! idvec {
    ()=> { Vec<u64> }
}

#[macro_export]
macro_rules! nmap {
    ()=> { HashMap::new() }
}