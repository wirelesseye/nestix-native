use std::{cell::Cell, rc::Rc};

use gtk4::{Fixed, Widget, glib};
use nestix::{Placement, Shared};
use nestix_native_core::TreeContext;
use taffy::NodeId;

type AddChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(&Widget, Option<NodeId>, Option<Widget>)>;
type RemoveChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;

/// Coalesces Taffy refreshes requested during one GTK main-loop turn.
pub(crate) struct LayoutRefreshContext {
    tree_context: Rc<TreeContext>,
    refresh_queued: Cell<bool>,
}

impl LayoutRefreshContext {
    pub fn new(tree_context: Rc<TreeContext>) -> Rc<Self> {
        Rc::new(Self {
            tree_context,
            refresh_queued: Cell::new(false),
        })
    }

    pub fn queue_refresh(self: &Rc<Self>) {
        if self.refresh_queued.replace(true) {
            return;
        }

        let this = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            this.flush_queued_refresh();
        });
    }

    pub fn flush_queued_refresh(&self) {
        if self.refresh_queued.replace(false) {
            self.tree_context.refresh();
        }
    }
}

/// Connects a GTK widget to the fixed-position host used by the Taffy tree.
pub(crate) struct ParentContext {
    pub fixed: Option<Fixed>,
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}

impl ParentContext {
    pub fn place_child(&self, child: &Widget, child_node: Option<NodeId>, placement: &Placement) {
        if let Some(insert_child) = &self.insert_child {
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<Widget>().cloned());
            insert_child(child, child_node, predecessor);
        } else if let Some(add_child) = &self.add_child {
            add_child(child, child_node);
        }
    }
}
