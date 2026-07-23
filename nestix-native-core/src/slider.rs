use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

/// Properties for a numeric slider.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SliderProps {
    /// Style classes applied to the slider.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether the slider accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,

    /// Current slider value.
    #[props(default = 0.0)]
    pub value: f64,

    /// Smallest permitted value.
    #[props(default = 0.0)]
    pub minimum: f64,

    /// Largest permitted value.
    #[props(default = 100.0)]
    pub maximum: f64,

    /// Called with the new value after a user change.
    pub on_value_change: Option<Shared<dyn Fn(f64)>>,
}
