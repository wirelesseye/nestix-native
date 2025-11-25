pub mod app;
pub mod button;
pub mod input;
pub mod label;
pub mod view;
pub mod window;

use std::rc::Rc;

pub use app::*;
pub use button::*;
pub use input::*;
pub use label::*;
pub use view::*;
pub use window::*;

use nestix_native_appkit::AppkitBackend;
use nestix_native_core::Backend;

pub fn default_backend() -> Rc<dyn Backend> {
    Rc::new(AppkitBackend)
}

#[derive(Clone)]
pub struct BackendContext {
    backend: Rc<dyn Backend>,
}
