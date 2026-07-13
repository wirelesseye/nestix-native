use nestix::{Element, component, components::ContextProvider, layout};

pub use nestix_native_core::RootProps;

use crate::{BackendContext, default_backend};

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    let backend = if let Some(ctx) = element.context::<BackendContext>() {
        ctx.backend
    } else {
        default_backend()
    };

    let platform_root = backend.create_root(props.clone());

    layout! {
        ContextProvider<BackendContext>(
            BackendContext { backend },
        ) {
            $(platform_root),
        }
    }
}
