use nestix::{Shared, props};

use crate::{ExtendsViewProps, ViewProps};

#[props]
#[derive(Debug, Clone)]
pub struct ButtonProps {
    #[props(extends(ExtendsViewProps))]
    view_props: ViewProps,

    pub title: String,

    pub on_click: Option<Shared<dyn Fn()>>,
}
