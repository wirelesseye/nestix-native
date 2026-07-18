use gtk4::{Fixed, Widget};
use nestix::Shared;
use taffy::NodeId;

type AddChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;
type InsertChild = Shared<dyn Fn(&Widget, Option<NodeId>, usize)>;
type RemoveChild = Shared<dyn Fn(&Widget, Option<NodeId>)>;

/// Connects a GTK widget to the fixed-position host used by the Taffy tree.
pub(crate) struct ParentContext {
    pub fixed: Option<Fixed>,
    pub add_child: Option<AddChild>,
    pub insert_child: Option<InsertChild>,
    pub remove_child: Option<RemoveChild>,
    pub parent_node: Option<NodeId>,
}
