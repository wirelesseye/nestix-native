use nestix::{Shared, props};

use crate::{ClassList, FontProps, ViewProps};

/// Properties for a checkbox control.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct CheckboxProps {
    /// Style classes applied to the checkbox.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Font and text presentation properties.
    #[props(nested, default)]
    pub font: FontProps,

    /// Text displayed beside the checkbox.
    #[props(start)]
    pub title: String,

    /// Whether the checkbox accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,

    /// Whether the checkbox is checked.
    #[props(default)]
    pub checked: bool,

    /// Called with the new checked state after a user change.
    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}
