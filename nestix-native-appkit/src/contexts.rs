use nestix::{Element, Shared};
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

/// Returns the insertion index among host-rendered siblings. Nestix placement
/// indices also count invisible components such as MenuBar, while AppKit and
/// Taffy only contain children that provide a native handle.
pub(crate) fn native_child_index(element: &Element) -> usize {
    element
        .previous_siblings()
        .into_iter()
        .map(|sibling| native_handle_count(&sibling))
        .sum()
}

fn native_handle_count(element: &Element) -> usize {
    if element.handle().is_some() {
        1
    } else {
        element.children().iter().map(native_handle_count).sum()
    }
}
