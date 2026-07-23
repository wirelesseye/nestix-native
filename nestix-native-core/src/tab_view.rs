use nestix::{Element, Layout, props};

use crate::{ClassList, ViewProps};

/// Properties for a tabbed container.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    /// Style classes applied to the tab view.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Tab items displayed by the container.
    #[props(default)]
    pub children: Layout,
}

/// Properties for one page in a [`TabViewProps`] container.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    /// Style classes applied to the tab page.
    #[props(default)]
    pub class: ClassList,

    /// Stable identifier for the tab.
    pub id: String,
    /// User-visible tab title.
    #[props(default)]
    pub title: String,
    /// Optional content displayed when the tab is active.
    pub children: Option<Element>,
}
