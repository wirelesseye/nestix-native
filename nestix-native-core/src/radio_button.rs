use nestix::{Shared, props};

use crate::{ClassList, FontProps, ViewProps};

/// Properties for a radio-button control.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct RadioButtonProps {
    /// Style classes applied to the radio button.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Font and text presentation properties.
    #[props(nested, default)]
    pub font: FontProps,

    /// Text displayed beside the radio button.
    #[props(start)]
    pub title: String,

    /// Whether the control accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,

    /// Identifier shared by mutually exclusive radio buttons.
    pub group: String,

    /// Whether this radio button is selected.
    #[props(default)]
    pub selected: bool,

    /// Called when the user selects this radio button.
    pub on_select: Option<Shared<dyn Fn()>>,
}
