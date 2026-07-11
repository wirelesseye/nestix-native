use nestix::Shared;
use taffy::NodeId;
use windows::Win32::Foundation::HWND;

type AddChild = Shared<dyn Fn(HWND, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(HWND, Option<NodeId>, usize)>;
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
