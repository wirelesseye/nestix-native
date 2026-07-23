use nestix::{Layout, Shared, props};

use crate::{ClassList, ViewProps};

/// Properties for a selection control.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SelectProps {
    /// Style classes applied to the control.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether the control accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,

    /// Value of the currently selected option.
    #[props(default)]
    pub value: Option<String>,

    /// Called with a newly selected option value.
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,

    /// Options displayed by the control.
    #[props(default)]
    pub children: Layout,
}

/// Properties for one option in a [`SelectProps`] control.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SelectOptionProps {
    /// User-visible option label.
    #[props(start)]
    pub label: String,

    /// Stable value used by the parent [`SelectProps`]. Values must be unique
    /// among sibling options.
    pub value: String,

    /// Whether this option may be selected.
    #[props(default = true)]
    pub enabled: bool,
}
