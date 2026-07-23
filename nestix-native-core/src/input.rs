use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

/// Properties for a single-line text input.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct InputProps {
    /// Style classes applied to the input.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Current text value.
    #[props(default)]
    pub value: String,

    /// Called with the new text after a user edit.
    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
