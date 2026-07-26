#[cfg(feature = "taffy")]
mod taffy {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    use nestix::{Readonly, State, create_state};
    use taffy::{NodeId, Size, Style, TaffyTree};

    pub struct TreeContext {
        tree: RefCell<TaffyTree>,
        root_node: Cell<Option<NodeId>>,
        node_layouts: RefCell<HashMap<NodeId, State<taffy::Layout>>>,
        layout_revision: State<u64>,
        refresh_request_revision: State<u64>,
        defer_refreshes: Cell<bool>,
        batch_depth: Cell<usize>,
        refresh_pending: Cell<bool>,
    }

    /// Parent-local native child order and its Taffy projection.
    pub struct ChildOrder<K> {
        entries: Vec<(K, Option<NodeId>)>,
    }

    impl<K: Clone + Eq> ChildOrder<K> {
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        pub fn place(&mut self, key: K, node: Option<NodeId>, predecessor: Option<K>) {
            let previous_index = self.entries.iter().position(|(entry, _)| entry == &key);
            self.entries.retain(|(entry, _)| entry != &key);
            let index = match predecessor {
                None => 0,
                Some(predecessor) => self
                    .entries
                    .iter()
                    .position(|(entry, _)| entry == &predecessor)
                    .map(|index| index + 1)
                    .unwrap_or_else(|| {
                        previous_index
                            .unwrap_or(self.entries.len())
                            .min(self.entries.len())
                    }),
            };
            self.entries.insert(index, (key, node));
        }

        pub fn remove(&mut self, key: K) {
            self.entries.retain(|(entry, _)| entry != &key);
        }

        pub fn last_key(&self) -> Option<K> {
            self.entries.last().map(|(key, _)| key.clone())
        }

        pub fn taffy_nodes(&self) -> Vec<NodeId> {
            self.entries.iter().filter_map(|(_, node)| *node).collect()
        }
    }

    impl TreeContext {
        pub fn new() -> Self {
            Self {
                tree: RefCell::new(TaffyTree::new()),
                root_node: Cell::new(None),
                node_layouts: RefCell::new(HashMap::new()),
                layout_revision: create_state(0),
                refresh_request_revision: create_state(0),
                defer_refreshes: Cell::new(false),
                batch_depth: Cell::new(0),
                refresh_pending: Cell::new(false),
            }
        }

        pub fn root_node(&self) -> Option<NodeId> {
            self.root_node.get()
        }

        pub fn set_root_node(&self, node: Option<NodeId>) {
            self.root_node.set(node);
        }

        pub fn create_node(&self, leaf: bool) -> NodeId {
            let node_id = if leaf {
                self.tree.borrow_mut().new_leaf(Style::default()).unwrap()
            } else {
                self.tree
                    .borrow_mut()
                    .new_with_children(Style::default(), &[])
                    .unwrap()
            };
            self.node_layouts
                .borrow_mut()
                .insert(node_id, create_state(taffy::Layout::default()));
            node_id
        }

        pub fn add_child(&self, parent: NodeId, child: NodeId) {
            self.tree.borrow_mut().add_child(parent, child).unwrap();
        }

        pub fn set_children(&self, parent: NodeId, children: &[NodeId]) {
            self.tree
                .borrow_mut()
                .set_children(parent, children)
                .unwrap();
        }

        pub fn remove_child(&self, parent: NodeId, child: NodeId) {
            self.tree.borrow_mut().remove_child(parent, child).unwrap();
        }

        /// Signal setter
        pub fn set_layout(&self, node: NodeId, layout: taffy::Layout) {
            let state = self.node_layouts.borrow_mut().get(&node).unwrap().clone();
            state.set(layout);
        }

        /// Returns a signal incremented after each completed layout pass.
        pub fn layout_revision(&self) -> Readonly<u64> {
            self.layout_revision.clone().into_readonly()
        }

        /// Signal incremented when a deferred layout pass is first requested.
        pub fn refresh_request_revision(&self) -> Readonly<u64> {
            self.refresh_request_revision.clone().into_readonly()
        }

        /// Defers layout computation until [`Self::flush_refresh`] is called.
        pub fn set_defer_refreshes(&self, defer: bool) {
            self.defer_refreshes.set(defer);
        }

        /// Signal getter
        pub fn layout(&self, node: NodeId) -> Option<taffy::Layout> {
            self.node_layouts
                .borrow()
                .get(&node)
                .map(|state| state.get())
        }

        pub fn update_style(&self, node: NodeId, updater: impl FnOnce(Style) -> Style) {
            let prev_style = {
                let tree = self.tree.borrow();
                tree.style(node).unwrap().clone()
            };
            let next_style = updater(prev_style);
            self.tree.borrow_mut().set_style(node, next_style).unwrap();
        }

        pub fn refresh(&self) {
            if self.batch_depth.get() > 0 || self.defer_refreshes.get() {
                if !self.refresh_pending.replace(true) && self.defer_refreshes.get() {
                    self.refresh_request_revision
                        .update(|revision| revision.wrapping_add(1));
                }
                return;
            }
            if let Some(root_node) = self.root_node() {
                self.update_node(root_node);
            }
        }

        /// Performs a pending deferred layout pass, if any.
        pub fn flush_refresh(&self) {
            if !self.refresh_pending.replace(false) {
                return;
            }
            if let Some(root_node) = self.root_node() {
                self.update_node(root_node);
            }
        }

        /// Defers refreshes until the matching [`Self::end_batch`] call.
        pub fn begin_batch(&self) {
            self.batch_depth.set(self.batch_depth.get() + 1);
        }

        /// Ends a refresh batch and performs at most one pending layout pass.
        pub fn end_batch(&self) {
            let depth = self.batch_depth.get();
            assert!(depth > 0, "layout batch underflow");
            self.batch_depth.set(depth - 1);
            if depth == 1 && self.refresh_pending.replace(false) {
                if self.defer_refreshes.get() {
                    self.refresh_pending.set(true);
                } else {
                    self.refresh();
                }
            }
        }

        fn update_node(&self, node: NodeId) {
            {
                let mut tree = self.tree.borrow_mut();
                tree.compute_layout(node, Size::max_content()).unwrap();
            }
            self.update_node_recursive(node);
            self.layout_revision
                .update(|revision| revision.wrapping_add(1));
        }

        fn update_node_recursive(&self, node: NodeId) {
            let layout = {
                let tree = self.tree.borrow();
                tree.layout(node).unwrap().clone()
            };
            self.set_layout(node, layout);

            let children = {
                let tree = self.tree.borrow();
                tree.children(node).unwrap()
            };
            for child in children {
                self.update_node_recursive(child);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn child_order_projects_only_layout_nodes() {
            let context = TreeContext::new();
            let first = context.create_node(true);
            let second = context.create_node(true);
            let mut order = ChildOrder::new();

            order.place("menu", None, None);
            order.place("first", Some(first), Some("menu"));
            order.place("second", Some(second), Some("first"));
            assert_eq!(order.taffy_nodes(), vec![first, second]);

            order.place("second", Some(second), None);
            assert_eq!(order.taffy_nodes(), vec![second, first]);
            order.place("first", Some(first), Some("outside-parent"));
            assert_eq!(order.taffy_nodes(), vec![second, first]);
            order.remove("second");
            assert_eq!(order.taffy_nodes(), vec![first]);
        }

        #[test]
        fn layout_revision_advances_once_per_completed_pass() {
            let context = TreeContext::new();
            let root = context.create_node(false);
            context.set_root_node(Some(root));
            let revision = context.layout_revision();

            context.refresh();
            assert_eq!(revision.get(), 1);

            context.begin_batch();
            context.refresh();
            context.refresh();
            assert_eq!(revision.get(), 1);
            context.end_batch();
            assert_eq!(revision.get(), 2);
        }

        #[test]
        fn deferred_refreshes_coalesce_until_flushed() {
            let context = TreeContext::new();
            let root = context.create_node(false);
            context.set_root_node(Some(root));
            context.set_defer_refreshes(true);
            let request_revision = context.refresh_request_revision();
            let layout_revision = context.layout_revision();

            context.refresh();
            context.refresh();
            assert_eq!(request_revision.get(), 1);
            assert_eq!(layout_revision.get(), 0);

            context.flush_refresh();
            assert_eq!(layout_revision.get(), 1);
            context.flush_refresh();
            assert_eq!(layout_revision.get(), 1);
        }
    }
}

#[cfg(feature = "taffy")]
pub use taffy::*;
