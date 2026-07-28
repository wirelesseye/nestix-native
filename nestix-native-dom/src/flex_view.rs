use nestix::{Element, component, layout};
use nestix_native_core::{
    FlexViewProps, StyleContext, StyleScope, matched_style, resolved_flex_view_style,
};

use crate::{
    renderer::{mount_host, renderer},
    style_declarations::flex_styles,
};

/// DOM flex-layout container.
#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__dom_FlexView"];

    let renderer = renderer(element);
    let node = renderer.create_element("div");
    mount_host(element, renderer.clone(), node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(matched, props);
    element.scoped_effect({
        let renderer = renderer.clone();
        let effective_style = effective_style.clone();
        move || {
            renderer.replace_styles(
                node,
                flex_styles(
                    &effective_style.get().unwrap_or_default(),
                    renderer.scale_factor(),
                ),
            );
        }
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
