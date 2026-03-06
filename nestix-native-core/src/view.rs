use nestix::props;

use crate::Dimension;

#[props(debug, extensible(ExtendsViewProps))]
#[derive(Debug, Clone)]
pub struct ViewProps {
    #[props(default = Dimension::Auto)]
    pub left: Dimension,
    #[props(default = Dimension::Auto)]
    pub top: Dimension,

    #[props(default = Dimension::Auto)]
    pub width: Dimension,
    #[props(default = Dimension::Auto)]
    pub height: Dimension,

    #[props(default = 0.0)]
    pub grow: f32,
}
