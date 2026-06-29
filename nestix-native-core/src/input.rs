use nestix::{Shared, props};

use crate::{ViewProps, ViewPropsExt, ViewPropsWrapper};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct InputProps {
    #[props(extends(ViewPropsExt, ViewPropsWrapper))]
    view_props: ViewProps,

    #[props(default)]
    pub value: String,

    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
