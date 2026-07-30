use nestix::{Element, Layout, component, components::Fragment, layout, props};

use crate::BackendContext;

/// Properties for [`BackendOverride`].
#[props(debug)]
#[derive(Debug, Clone)]
pub struct BackendCaseProps {
    /// Stable identifier of the backend that receives the replacement layout.
    #[props(start)]
    pub backend_id: String,

    /// Layout rendered when the active backend matches [`Self::backend_id`].
    #[props(default)]
    pub replacement: Layout,

    /// Default layout rendered for all other backends.
    #[props(default)]
    pub children: Layout,
}

/// Replaces its default layout for one native backend.
///
/// `BackendOverride` must be mounted beneath [`crate::Root`]. Backend identifiers
/// are supplied by backend implementations through
/// [`crate::Backend::backend_id`], so third-party backends require no central
/// registration.
#[component]
pub fn BackendCase(props: &BackendCaseProps, element: &Element) -> Element {
    let backend = element
        .context::<BackendContext>()
        .expect("BackendCase must be mounted beneath Root")
        .backend;
    let selected = if backend.backend_id() == props.backend_id.get() {
        props.replacement.clone()
    } else {
        props.children.clone()
    };

    layout! {
        Fragment(.children = selected)
    }
}
