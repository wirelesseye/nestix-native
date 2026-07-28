use nestix::{Element, component, components::ContextProvider, layout};
use nestix_native_core::{RootProps, StyleScope};
use wasm_bindgen::JsCast;
use web_sys::Node;

use crate::{DomRendererContext, take_mount_target};

/// DOM implementation of the Nestix Native root.
#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__dom_Root"];

    let target = take_mount_target();
    let renderer = DomRendererContext::browser(target.unchecked_into::<Node>());
    element.provide_handle(renderer.renderer().root_handle());

    layout! {
        ContextProvider<DomRendererContext>(renderer) {
            StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                $(props.children.clone())
            }
        }
    }
}
