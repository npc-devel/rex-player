
#[macro_export]
macro_rules! asset {
    ($x:expr,$y:expr) => {
        {
            let temp_s = format!("/home/ppc/Dev/GitHub/rex-player/assets/{}.{}",$x,$y);
            temp_s
        }
    }
}

#[macro_export]
macro_rules! view {
    ($x:expr,$y:expr) => {
        {
            let temp_s = format!("/home/ppc/Dev/GitHub/rex-player/rex-player/src/views/{}.{}",$x,$y);
            println!("{temp_s}");
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
    ()=> { HashMap<u64,Nvisual> }
}

#[macro_export]
macro_rules! idvec {
    ()=> { Vec<u64> }
}

#[macro_export]
macro_rules! nmap {
    ()=> { HashMap::new() }
}