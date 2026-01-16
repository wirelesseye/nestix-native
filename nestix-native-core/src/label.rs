use nestix::props;

use crate::{ExtendsViewProps, ViewProps};

#[props]
#[derive(Debug, Clone)]
pub struct LabelProps {
    #[props(extends(ExtendsViewProps))]
    view_props: ViewProps,

    pub text: String,
}
