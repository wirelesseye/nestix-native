use nestix::{Element, component, layout};

pub use nestix_native_core::RootProps;

use crate::{BackendContext, BackendProvider, create_backend_element, default_backend};

/// Establishes the native backend context for a component tree.
///
/// Uses an inherited [`BackendContext`] when present, otherwise selecting the
/// default backend for the current platform.
#[component]
pub fn Root(props: &RootProps, element: &Element) -> Option<Element> {
    if element.context::<BackendContext>().is_some() {
        return create_backend_element(element, "Root", |backend| {
            backend.create_root(props.clone())
        });
    }

    let backend = default_backend();
    let platform_root = backend.create_root(props.clone());
    Some(layout! {
        BackendProvider(backend) {
            $(platform_root)
        }
    })
}
