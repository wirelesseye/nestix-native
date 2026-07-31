use nestix::{Layout, Shared, props};

use crate::{ClassList, ViewProps};

/// Properties for a navigation list intended for use inside a sidebar.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SidebarNavigationProps {
    /// Style classes applied to the navigation list.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Value of the currently selected navigation item.
    #[props(default)]
    pub value: Option<String>,

    /// Called with a newly selected navigation item value.
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,

    /// Navigation items displayed by the list.
    #[props(default)]
    pub children: Layout,
}

/// Properties for one item in a [`SidebarNavigationProps`] list.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct NavigationItemProps {
    /// User-visible item label.
    #[props(start)]
    pub label: String,

    /// Stable value used by the parent navigation list. Values must be unique
    /// among sibling items.
    pub value: String,

    /// Whether this item may be selected.
    #[props(default = true)]
    pub enabled: bool,
}
