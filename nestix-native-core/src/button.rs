use nestix::{Shared, props};

use crate::{Appearance, ClassList, ContainerProps, FontProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(nested, default = ContainerProps::auto_padding())]
    pub container: ContainerProps,

    #[props(nested, default)]
    pub font: FontProps,

    #[props(default)]
    pub appearance: Appearance,

    #[props(default)]
    pub title: String,

    #[props(default)]
    pub disabled: bool,

    pub on_click: Option<Shared<dyn Fn()>>,
}
