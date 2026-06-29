use nestix::props;

use crate::{ViewProps, ViewPropsExt, ViewPropsWrapper};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TextProps {
    #[props(extends(ViewPropsExt, ViewPropsWrapper))]
    view_props: ViewProps,
    #[props(start)]
    pub text: String,
}
