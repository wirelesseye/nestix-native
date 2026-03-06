use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use nestix::{Shared, State, create_state};
use objc2_foundation::NSObject;
use taffy::{NodeId, Size, Style, TaffyTree};

pub(crate) struct TreeContext {
    tree: RefCell<TaffyTree>,
    root_node: Cell<Option<NodeId>>,
    node_layouts: RefCell<HashMap<NodeId, State<taffy::Layout>>>,
}

impl TreeContext {
    pub fn new() -> Self {
        Self {
            tree: RefCell::new(TaffyTree::new()),
            root_node: Cell::new(None),
            node_layouts: RefCell::new(HashMap::new()),
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

    pub fn remove_child(&self, parent: NodeId, child: NodeId) {
        self.tree.borrow_mut().remove_child(parent, child).unwrap();
    }

    pub fn set_layout(&self, node: NodeId, layout: taffy::Layout) {
        let state = self.node_layouts.borrow_mut().get(&node).unwrap().clone();
        state.set(layout);
    }

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

    pub fn update(&self) {
        if let Some(root_node) = self.root_node() {
            self.update_node(root_node);
        }
    }

    fn update_node(&self, node: NodeId) {
        {
            let mut tree = self.tree.borrow_mut();
            tree.compute_layout(node, Size::max_content()).unwrap();
        }
        self.update_node_recursive(node);
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

pub(crate) struct ParentContext {
    pub add_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub remove_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub parent_node: Option<NodeId>,
}
