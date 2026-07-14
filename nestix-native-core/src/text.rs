use nestix::props;

use crate::{ClassList, FontProps, ViewProps};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct TextProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(nested, default)]
    pub font: FontProps,

    #[props(start)]
    pub text: String,
}
