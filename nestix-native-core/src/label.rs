use nestix::props;

use crate::{ViewPropsExt, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct LabelProps {
    #[props(extends(ViewPropsExt))]
    view_props: ViewProps,

    pub text: String,
}
