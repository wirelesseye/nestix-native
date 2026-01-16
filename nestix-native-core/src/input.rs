use nestix::{Shared, props};

use crate::{ExtendsViewProps, ViewProps};

#[props]
#[derive(Debug, Clone)]
pub struct InputProps {
    #[props(extends(ExtendsViewProps))]
    view_props: ViewProps,

    #[props(default)]
    pub value: String,

    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
