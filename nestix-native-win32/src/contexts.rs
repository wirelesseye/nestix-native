use nestix::{Placement, Shared};
use taffy::NodeId;
use windows::Win32::Foundation::HWND;

type AddChild = Shared<dyn Fn(HWND, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(HWND, Option<NodeId>, Option<HWND>)>;
type RemoveChild = Shared<dyn Fn(HWND, Option<NodeId>)>;

/// Connects a component to its native parent and to the parent's Taffy node.
/// Native child order and layout-tree order must be updated by the same callback.
pub(crate) struct ParentContext {
    pub parent_hwnd: HWND,
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}

impl ParentContext {
    pub fn place_child(&self, child: HWND, child_node: Option<NodeId>, placement: &Placement) {
        if let Some(insert_child) = &self.insert_child {
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<HWND>().copied());
            insert_child(child, child_node, predecessor);
        } else if let Some(add_child) = &self.add_child {
            add_child(child, child_node);
        }
    }
}
