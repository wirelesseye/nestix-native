use nestix::{Shared, props};

use crate::{Appearance, ClassList, ContainerProps, FontProps, ViewProps};

/// Properties for a push button.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    /// Style classes applied to the button.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Container padding properties.
    #[props(nested, default = ContainerProps::auto_padding())]
    pub container: ContainerProps,

    /// Font and text presentation properties.
    #[props(nested, default)]
    pub font: FontProps,

    /// Controls whether native or custom styling is used.
    #[props(default)]
    pub appearance: Appearance,

    /// Text displayed by the button.
    #[props(default)]
    pub title: String,

    /// Whether the button rejects user interaction.
    #[props(default)]
    pub disabled: bool,

    /// Called when the user activates the button.
    pub on_click: Option<Shared<dyn Fn()>>,
}
