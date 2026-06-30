use nestix::props;

use crate::ViewProps;

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TextProps {
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(start)]
    pub text: String,
}
