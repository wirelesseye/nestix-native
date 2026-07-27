use nestix::{Element, component};
use nestix_native_core::{
    StyleContext, TextProps, matched_style, resolve_font_props, resolved_view_style,
};
use wasm_bindgen::JsCast;
use web_sys::Node;

use crate::{
    dom::{create_html_element, mount_host},
    style::{apply_font, apply_view_style},
};

/// DOM text label.
#[component]
pub fn Text(props: &TextProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__dom_Text"];

    let html = create_html_element("span");
    let node = html.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let html = html.clone();
        let effective_style = effective_style.clone();
        let text = props.text.clone();
        let font_family = props.font.font_family.clone();
        let font_size = props.font.font_size.clone();
        let font_weight = props.font.font_weight.clone();
        let font_style = props.font.font_style.clone();
        let text_color = props.font.text_color.clone();
        move || {
            let style = effective_style.get().unwrap_or_default();
            html.set_text_content(Some(&text.get()));
            apply_view_style(&html.style(), &style);
            apply_font(
                &html.style(),
                &resolve_font_props(
                    Some(&style),
                    font_family.get(),
                    font_size.get(),
                    font_weight.get(),
                    font_style.get(),
                    text_color.get(),
                ),
            );
        }
    });
}
