use nestix::{Element, component};
use nestix_native_core::{
    ButtonProps, StyleContext, matched_style, resolve_font_props, resolved_view_style,
    style_appearance, style_padding_with_default,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, HtmlButtonElement, Node};

use crate::{
    dom::{create_html_element, mount_host},
    style::{apply_appearance, apply_font, apply_padding, apply_view_style},
};

/// DOM push button.
#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__dom_Button"];

    let button = create_html_element("button")
        .dyn_into::<HtmlButtonElement>()
        .expect("button element must be an HtmlButtonElement");
    let node = button.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let on_click = props.on_click.clone();
    let listener = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(on_click) = on_click.get() {
            on_click();
        }
    });
    button.set_onclick(Some(listener.as_ref().unchecked_ref()));
    element.on_unmount({
        let button = button.clone();
        move || {
            button.set_onclick(None);
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
        let button = button.clone();
        let effective_style = effective_style.clone();
        let title = props.title.clone();
        let disabled = props.disabled.clone();
        let appearance = props.appearance.clone();
        let padding = props.container.padding();
        let font_family = props.font.font_family.clone();
        let font_size = props.font.font_size.clone();
        let font_weight = props.font.font_weight.clone();
        let font_style = props.font.font_style.clone();
        let text_color = props.font.text_color.clone();
        move || {
            let mut style = effective_style.get().unwrap_or_default();
            style.appearance = Some(style_appearance(Some(&style), appearance.get()));
            let padding = style_padding_with_default(
                Some(&style),
                padding.get(),
                nestix_native_core::WithAuto::Auto,
            );
            style.padding_left = Some(padding.left);
            style.padding_right = Some(padding.right);
            style.padding_top = Some(padding.top);
            style.padding_bottom = Some(padding.bottom);

            button.set_text_content(Some(&title.get()));
            button.set_disabled(disabled.get());
            apply_view_style(&button.style(), &style);
            apply_padding(&button.style(), &style);
            apply_appearance(&button.style(), style.appearance.unwrap_or_default());
            apply_font(
                &button.style(),
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
