use nestix::{Shared, props};

use crate::{ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(nested, default)]
    pub view: ViewProps,
    
    #[props(default)]
    pub title: String,

    pub on_click: Option<Shared<dyn Fn()>>,
}
