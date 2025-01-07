include!("utils/macros.rs");
include!("app/n_app.rs");

fn main() {
    let mut napp = Napp::new(1280,720);
    napp.prepare();
    napp.run();
    napp.clean_up();
}