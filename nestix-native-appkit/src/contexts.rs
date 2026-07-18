use nestix::{Placement, Shared};
use objc2_foundation::NSObject;
use taffy::NodeId;

type AddChild = Shared<dyn Fn(&NSObject, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(&NSObject, Option<NodeId>, Option<*const NSObject>)>;
type RemoveChild = Shared<dyn Fn(&NSObject, Option<NodeId>)>;

/// Connects a component to its native parent and to the parent's Taffy node.
/// The object is intentionally erased here; component modules retain concrete AppKit types.
pub(crate) struct ParentContext {
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}

impl ParentContext {
    pub fn place_child(&self, child: &NSObject, child_node: Option<NodeId>, placement: &Placement) {
        if let Some(insert_child) = &self.insert_child {
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<*const NSObject>().copied());
            insert_child(child, child_node, predecessor);
        } else if let Some(add_child) = &self.add_child {
            add_child(child, child_node);
        }
    }
}
