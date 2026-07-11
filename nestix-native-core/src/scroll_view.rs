use nestix::{Element, props};

use crate::{ClassList, ViewProps};

/// The common properties supported by a scrollable container.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ScrollViewProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default = true)]
    pub scroll_x: bool,

    #[props(default = true)]
    pub scroll_y: bool,

    pub children: Option<Element>,
}
