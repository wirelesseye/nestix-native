use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct SliderProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default = true)]
    pub enabled: bool,

    #[props(default = 0.0)]
    pub value: f64,

    #[props(default = 0.0)]
    pub minimum: f64,

    #[props(default = 100.0)]
    pub maximum: f64,

    pub on_value_change: Option<Shared<dyn Fn(f64)>>,
}
