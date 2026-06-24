use nestix::{Shared, props};

use crate::{ViewPropsExt, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(extends(ViewPropsExt))]
    view_props: ViewProps,

    pub title: String,

    pub on_click: Option<Shared<dyn Fn()>>,
}
