use nestix::{Layout, Shared, props};

use crate::{ClassList, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct SelectProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default = true)]
    pub enabled: bool,

    #[props(default)]
    pub value: Option<String>,

    pub on_value_change: Option<Shared<dyn Fn(&str)>>,

    #[props(default)]
    pub children: Layout,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct SelectOptionProps {
    #[props(start)]
    pub label: String,

    /// Stable value used by the parent [`SelectProps`]. Values must be unique
    /// among sibling options.
    pub value: String,

    #[props(default = true)]
    pub enabled: bool,
}
