use nestix::{Shared, props};

use crate::{ViewPropsExt, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct InputProps {
    #[props(extends(ViewPropsExt))]
    view_props: ViewProps,

    #[props(default)]
    pub value: String,

    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
