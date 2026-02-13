use nestix::{Element, component, components::ContextProvider, layout};

pub use nestix_native_core::RootProps;

use crate::{BackendContext, default_backend};

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    let backend = if let Some(ctx) = element.context::<BackendContext>() {
        ctx.backend.clone()
    } else {
        default_backend()
    };

    let platform_root = backend.create_root(props.clone());

    if let Some(platform_root) = &platform_root {
        element.forward_handle(platform_root);
    }

    layout! {
        ContextProvider<BackendContext>(
            .value = BackendContext {
                backend
            },
        ) {
            $(platform_root),
        }
    }
}
