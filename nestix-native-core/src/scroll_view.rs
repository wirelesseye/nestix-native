use nestix::{Element, props};

use crate::{ClassList, ViewProps};

/// The common properties supported by a scrollable container.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ScrollViewProps {
    /// Style classes applied to the scroll view.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether horizontal scrolling is enabled.
    #[props(default = true)]
    pub scroll_x: bool,

    /// Whether vertical scrolling is enabled.
    #[props(default = true)]
    pub scroll_y: bool,

    /// Optional content displayed inside the scrolling viewport.
    pub children: Option<Element>,
}
