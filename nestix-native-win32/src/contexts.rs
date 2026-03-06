use nestix::Shared;
use taffy::NodeId;
use windows::Win32::Foundation::HWND;

pub struct ParentContext {
    pub parent_hwnd: HWND,
    pub add_child: Option<Shared<dyn Fn(HWND, Option<NodeId>)>>,
    pub remove_child: Option<Shared<dyn Fn(HWND, Option<NodeId>)>>,
    pub parent_node: Option<NodeId>,
}
