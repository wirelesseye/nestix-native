use nestix::{Element, Layout, PropValue, component, components::ContextProvider, layout};
use nestix_native_core::Backend;

pub use nestix_native_core::DomSurfaceProps;

use crate::{BackendContext, dom::DOM_BACKEND};

/// Embeds a managed DOM document and renders descendants with the DOM backend.
#[component]
pub fn DomSurface(props: &DomSurfaceProps, element: &Element) -> Option<Element> {
    let host_backend = element
        .context::<BackendContext>()
        .expect("DomSurface must be mounted beneath Root")
        .backend;

    assert_ne!(
        host_backend.backend_id(),
        DOM_BACKEND.backend_id(),
        "DomSurface cannot be nested inside a DOM backend"
    );

    let managed_children = layout! {
        ContextProvider<BackendContext>(BackendContext { backend: &DOM_BACKEND }) {
            nestix_native_dom::DomDocumentRoot(.children = props.children.clone())
        }
    };
    let mut host_props = props.clone();
    host_props.children = PropValue::from_plain(Layout::from(managed_children));
    Some(
        host_backend
            .create_dom_surface(host_props)
            .unwrap_or_else(|| {
                panic!(
                    "backend `{}` does not support DomSurface",
                    host_backend.backend_id()
                )
            }),
    )
}
