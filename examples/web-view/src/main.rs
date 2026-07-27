#[cfg(target_os = "macos")]
mod app;

#[cfg(target_os = "macos")]
fn main() {
    use env_logger::Env;
    use nestix::{layout, mount_root};

    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { app::App });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the web-view example currently supports macOS and browser WebAssembly");
}
