use nestix::{Shared, props};

use crate::{ClassList, FontProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(nested, default)]
    pub font: FontProps,

    #[props(default)]
    pub title: String,

    pub on_click: Option<Shared<dyn Fn()>>,
}
