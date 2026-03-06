use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use nestix::{Shared, State, create_state};
use objc2_foundation::NSObject;
use taffy::{NodeId, Size, Style, TaffyTree};

pub(crate) struct ParentContext {
    pub add_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub remove_child: Option<Shared<dyn Fn(&NSObject, Option<NodeId>)>>,
    pub parent_node: Option<NodeId>,
}
