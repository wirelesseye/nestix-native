use nestix::{Layout, props};

use crate::{ClassList, ViewProps};

/// Properties for a managed DOM subtree embedded in a native view.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DomSurfaceProps {
    /// Style classes applied to the native surface.
    #[props(default)]
    pub class: ClassList,

    /// Common outer layout properties. The surface is one native layout leaf.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether the managed document has a transparent background.
    #[props(default = true)]
    pub transparent: bool,

    /// Components rendered into the managed DOM document.
    #[props(default)]
    pub children: Layout,
}
