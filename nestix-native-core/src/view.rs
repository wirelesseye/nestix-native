use nestix::props;

use crate::Length;

#[props(extensible(ExtendsViewProps))]
#[derive(Debug, Clone)]
pub struct ViewProps {
    #[props(default = Length::Auto)]
    pub x: Length,
    #[props(default = Length::Auto)]
    pub y: Length,

    #[props(default = Length::Auto)]
    pub width: Length,
    #[props(default = Length::Auto)]
    pub height: Length,
}
