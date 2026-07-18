use nestix::{Element, Shared};
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

/// Returns the nearest preceding host handle from the same Nestix list,
/// skipping logical siblings that do not render a host object.
pub(crate) fn native_predecessor(element: &Element) -> Option<*const NSObject> {
    element
        .previous_siblings()
        .into_iter()
        .find_map(|sibling| sibling.last_handle())
        .and_then(|handle| handle.downcast_ref::<*const NSObject>().copied())
}
