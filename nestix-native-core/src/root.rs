use nestix::{Layout, props};

use crate::ClassList;

#[props(debug)]
#[derive(Debug, Clone)]
pub struct RootProps {
    #[props(default)]
    pub class: ClassList,

    #[props(default)]
    pub children: Layout,
}
