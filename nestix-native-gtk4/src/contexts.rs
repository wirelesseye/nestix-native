use gtk4::{Fixed, Widget};
use nestix::{Placement, Shared};
use taffy::NodeId;

type AddChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(&Widget, Option<NodeId>, Option<Widget>)>;
type RemoveChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;

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
