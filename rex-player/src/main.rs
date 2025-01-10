include!("macros.rs");
include!("app.rs");

fn main() {
    let mut napp = App::new(1280,720);
    napp.prepare();
    napp.run();
    napp.clean_up();
}