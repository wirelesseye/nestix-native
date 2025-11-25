pub mod list_view;
pub mod stack_view;
pub mod tab_view;

pub use list_view::*;
pub use stack_view::*;
pub use tab_view::*;

use nestix::Shared;
use objc2_foundation::NSObject;

#[derive(Clone)]
pub struct ParentContext {
    pub add_child: Option<Shared<dyn Fn(&NSObject)>>,
}
