mod app;

fn main() {
    use env_logger::Env;
    use nestix::{layout, mount_root};

    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { app::App });
}
