use nestix::{Shared, props};

use crate::{ViewProps, ViewPropsExt, ViewPropsWrapper};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(extends(ViewPropsExt, ViewPropsWrapper))]
    view_props: ViewProps,
    
    #[props(default)]
    pub title: String,

    pub on_click: Option<Shared<dyn Fn()>>,
}
