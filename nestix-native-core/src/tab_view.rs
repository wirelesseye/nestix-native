use nestix::{Element, Layout, props};

use crate::{ExtendsViewProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    #[props(extends(ExtendsViewProps))]
    view_props: ViewProps,
    
    #[props(default)]
    pub children: Layout,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    pub id: String,
    #[props(default)]
    pub title: String,
    pub children: Option<Element>,
}
