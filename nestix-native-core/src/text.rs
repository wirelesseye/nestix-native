use nestix::props;

use crate::{ClassList, FontProps, ViewProps};

/// Properties for a text label.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TextProps {
    /// Style classes applied to the label.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Font and text presentation properties.
    #[props(nested, default)]
    pub font: FontProps,

    /// Text displayed by the label.
    #[props(start)]
    pub text: String,
}
