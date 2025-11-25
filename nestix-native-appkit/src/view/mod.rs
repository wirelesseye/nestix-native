pub mod list_view;
pub mod stack_view;
pub mod tab_view;

pub use list_view::*;
use objc2::rc::Retained;
pub use stack_view::*;
pub use tab_view::*;

use nestix::Shared;
use objc2_foundation::NSObject;

#[derive(Clone)]
pub struct ParentContext {
    pub ns_object: Option<Retained<NSObject>>,
    pub add_child: Option<Shared<dyn Fn(&NSObject)>>,
}
