use nestix::{Element, Layout, props};

use crate::{ClassList, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default)]
    pub children: Layout,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    #[props(default)]
    pub class: ClassList,

    pub id: String,
    #[props(default)]
    pub title: String,
    pub children: Option<Element>,
}
