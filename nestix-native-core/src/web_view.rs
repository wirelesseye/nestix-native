use nestix::props;

use crate::{ClassList, ViewProps};

/// Properties for a view that displays web content from a URL.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct WebViewProps {
    /// Style classes applied to the web view.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// URL displayed by the web view.
    #[props(start)]
    pub url: String,
}
