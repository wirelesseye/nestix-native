use nestix::{Shared, props};

use crate::{ClassList, FontProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct RadioButtonProps {
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

    pub group: String,

    #[props(default)]
    pub selected: bool,

    pub on_select: Option<Shared<dyn Fn()>>,
}
