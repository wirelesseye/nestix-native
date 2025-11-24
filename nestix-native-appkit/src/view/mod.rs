pub mod list_view;
pub mod stack_view;

pub use list_view::*;
pub use stack_view::*;

use nestix::Shared;
use objc2_app_kit::NSView;

#[derive(Clone)]
pub struct ParentViewContext {
    pub add_child: Shared<dyn Fn(&NSView)>,
}
