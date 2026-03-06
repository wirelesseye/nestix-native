use nestix::Shared;
use objc2_foundation::NSObject;
use taffy::NodeId;

pub(crate) struct ParentContext {
    pub add_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub remove_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub parent_node: Option<NodeId>,
}
