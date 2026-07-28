#![allow(dead_code, non_snake_case)]

#[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
use nestix::{Element, component, layout};
#[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
use nestix_native::{Button, DomSurface, DomTemplate, FlexView, WebViewController, Window};

#[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
#[component]
fn MixedControls() -> Element {
    let controller = WebViewController::new();
    layout! {
        Window {
            FlexView {
                Button(.title = "Native")
                DomSurface(
                    .view(.width = 320, .height = 180),
                    .transparent = false,
                    .inspectable = true,
                    .controller = controller,
                    .template = DomTemplate::html(
                        "<!doctype html><body><main data-nestix-root></main></body>",
                    ),
                ) {
                    Button(.title = "DOM")
                }
            }
        }
    }
}

#[test]
fn dom_surface_compiles_through_layout() {
    #[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
    let _mixed = layout! { MixedControls };
}
