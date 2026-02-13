pub mod button;
pub mod input;
pub mod label;
pub mod linear_view;
pub mod root;
pub mod stack_view;
pub mod tab_view;
pub mod window;

pub use button::*;
pub use input::*;
pub use label::*;
pub use linear_view::*;
pub use root::*;
pub use stack_view::*;
pub use tab_view::*;
pub use window::*;

pub use nestix_native_core::*;

use std::rc::Rc;

#[cfg(target_os = "macos")]
pub fn default_backend() -> Rc<dyn Backend> {
    Rc::new(nestix_native_appkit::AppkitBackend)
}

#[cfg(target_os = "windows")]
pub fn default_backend() -> Rc<dyn Backend> {
    Rc::new(nestix_native_win32::Win32Backend)
}

#[derive(Clone)]
pub struct BackendContext {
    backend: Rc<dyn Backend>,
}
