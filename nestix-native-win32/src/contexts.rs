use std::rc::Rc;

use nestix::Placement;
use taffy::NodeId;
use windows::Win32::Foundation::HWND;

use crate::surface::{VisualHandle, VisualId, VisualSurface};

/// Connects a component to its logical parent and nearest native surface.
pub(crate) struct ParentContext {
    pub surface: Rc<VisualSurface>,
    pub parent_visual: Option<VisualId>,
    pub parent_node: Option<NodeId>,
}

impl ParentContext {
    pub(crate) fn mount_virtual(&self, node: NodeId) -> VisualHandle {
        self.surface.mount(self.parent_visual, node, None)
    }

    pub(crate) fn mount_native(&self, node: NodeId, hwnd: HWND) -> VisualHandle {
        self.surface.mount(self.parent_visual, node, Some(hwnd))
    }

    pub(crate) fn place_child(&self, child: &VisualHandle, placement: &Placement) {
        let predecessor = placement
            .pred
            .as_ref()
            .and_then(|handle| self.surface.resolve_handle(handle));
        self.surface.place(child.id(), predecessor);
    }

    pub(crate) fn remove_child(&self, child: &VisualHandle) {
        self.surface.remove(child.id());
    }

    pub(crate) fn child_context(&self, child: &VisualHandle, node: NodeId) -> Self {
        Self {
            surface: child.surface().clone(),
            parent_visual: Some(child.id()),
            parent_node: Some(node),
        }
    }
}
