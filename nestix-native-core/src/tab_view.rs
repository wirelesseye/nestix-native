use nestix::{Element, Layout, props};

use crate::{ViewProps, ViewPropsExt, ViewPropsWrapper};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    #[props(extends(ViewPropsExt, ViewPropsWrapper))]
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
