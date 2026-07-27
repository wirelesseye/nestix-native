#![cfg(target_arch = "wasm32")]

mod app;

use std::mem;

use nestix::layout;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn start() {
    let app = layout! { app::App };
    nestix_native::dom::mount_root("#app", &app);
    mem::forget(app);
}
