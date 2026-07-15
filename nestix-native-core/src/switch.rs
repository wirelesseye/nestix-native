use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct SwitchProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default = true)]
    pub enabled: bool,

    #[props(default)]
    pub checked: bool,

    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}
