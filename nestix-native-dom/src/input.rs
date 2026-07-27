use nestix::{Element, component};
use nestix_native_core::{InputProps, StyleContext, matched_style, resolved_view_style};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, HtmlInputElement, Node};

use crate::{
    dom::{create_html_element, mount_host},
    style::apply_view_style,
};

/// DOM single-line text input.
#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__dom_Input"];

    let input = create_html_element("input")
        .dyn_into::<HtmlInputElement>()
        .expect("input element must be an HtmlInputElement");
    input.set_type("text");
    let node = input.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let on_text_change = props.on_text_change.clone();
    let event_input = input.clone();
    let listener = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(on_text_change) = on_text_change.get() {
            on_text_change(&event_input.value());
        }
    });
    input.set_oninput(Some(listener.as_ref().unchecked_ref()));
    element.on_unmount({
        let input = input.clone();
        move || {
            input.set_oninput(None);
            let _ = &listener;
        }
    });

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let input = input.clone();
        let effective_style = effective_style.clone();
        let value = props.value.clone();
        move || {
            let next = value.get();
            if input.value() != next {
                input.set_value(&next);
            }
            apply_view_style(&input.style(), &effective_style.get().unwrap_or_default());
        }
    });
}
