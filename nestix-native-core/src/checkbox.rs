use nestix::{Shared, props};

use crate::{ClassList, FontProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct CheckboxProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(nested, default)]
    pub font: FontProps,

    #[props(start)]
    pub title: String,

    #[props(default = true)]
    pub enabled: bool,

    #[props(default)]
    pub checked: bool,

    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}
