use nestix::{Element, component, components::ContextProvider, layout};

pub use nestix_native_core::AppProps;

use crate::{BackendContext, default_backend};

#[component]
pub fn App(props: &AppProps, element: &Element) -> Element {
    let backend = if let Some(ctx) = element.context::<BackendContext>() {
        ctx.backend.clone()
    } else {
        default_backend()
    };

    let platform_app = backend.create_app(props.clone());

    if let Some(platform_app) = &platform_app {
        element.forward_handle(platform_app);
    }

    layout! {
        ContextProvider<BackendContext>(
            .value = BackendContext {
                backend
            },
        ) {
            $option(platform_app),
        }
    }
}
