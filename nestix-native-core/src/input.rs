use nestix::{Shared, props};

use crate::ViewProps;

#[props(debug)]
#[derive(Debug, Clone)]
pub struct InputProps {
    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default)]
    pub value: String,

    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
