use nestix::props;

use crate::{ViewPropsExt, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TextProps {
    #[props(extends(ViewPropsExt))]
    view_props: ViewProps,
    #[props(start)]
    pub text: String,
}
