use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

/// Properties for an on/off switch.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SwitchProps {
    /// Style classes applied to the switch.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether the switch accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,

    /// Whether the switch is on.
    #[props(default)]
    pub checked: bool,

    /// Called with the new state after a user change.
    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}
