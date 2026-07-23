use nestix::{Layout, props};

use crate::ClassList;

/// Properties for the root of a native component tree.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct RootProps {
    /// Style classes applied to the root.
    #[props(default)]
    pub class: ClassList,

    /// Components mounted below the root.
    #[props(default)]
    pub children: Layout,
}
