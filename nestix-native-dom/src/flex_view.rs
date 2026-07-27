use nestix::{Element, component, layout};
use nestix_native_core::{
    FlexViewProps, StyleContext, StyleScope, matched_style, resolved_flex_view_style,
};
use wasm_bindgen::JsCast;
use web_sys::Node;

use crate::{dom::create_html_element, dom::mount_host, style::apply_flex_style};

/// DOM flex-layout container.
#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__dom_FlexView"];

    let html = create_html_element("div");
    let node = html.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(matched, props);
    element.scoped_effect({
        let html = html.clone();
        let effective_style = effective_style.clone();
        move || apply_flex_style(&html.style(), &effective_style.get().unwrap_or_default())
    });

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            $(props.children.clone())
        }
    }
}
