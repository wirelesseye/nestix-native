use std::rc::Rc;

use nestix::{Element, Shared, component};
use nestix_native_core::{InputProps, StyleContext, matched_style, resolved_view_style};

use crate::{
    DomEventData, DomEventOptions, DomValue,
    renderer::{mount_host, renderer},
    style_declarations::view_styles,
};

/// DOM single-line text input.
#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__dom_Input"];

    let renderer = renderer(element);
    let node = renderer.create_element("input");
    renderer.set_attribute(node, "type".to_string(), Some("text".to_string()));
    mount_host(element, renderer.clone(), node);

    let on_text_change = props.on_text_change.clone();
    renderer.listen(
        node,
        "input".to_string(),
        DomEventOptions::default(),
        Shared::from(Rc::new(move |event: &DomEventData| {
            if let Some(on_text_change) = on_text_change.get() {
                on_text_change(event.value.as_deref().unwrap_or_default());
            }
        }) as Rc<dyn Fn(&DomEventData)>),
    );

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let renderer = renderer.clone();
        let effective_style = effective_style.clone();
        let value = props.value.clone();
        let placeholder = props.placeholder.clone();
        move || {
            renderer.set_property(node, "value".to_string(), DomValue::String(value.get()));
            renderer.set_attribute(node, "placeholder".to_string(), Some(placeholder.get()));
            renderer.replace_styles(
                node,
                view_styles(
                    &effective_style.get().unwrap_or_default(),
                    renderer.scale_factor(),
                ),
            );
        }
    });
}
