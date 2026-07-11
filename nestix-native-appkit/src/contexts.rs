use nestix::Shared;
use objc2_foundation::NSObject;
use taffy::NodeId;

type AddChild = Shared<dyn Fn(&NSObject, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(&NSObject, Option<NodeId>, usize)>;
type RemoveChild = Shared<dyn Fn(&NSObject, Option<NodeId>)>;

/// Connects a component to its native parent and to the parent's Taffy node.
/// The object is intentionally erased here; component modules retain concrete AppKit types.
pub(crate) struct ParentContext {
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}
