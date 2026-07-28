use std::rc::Rc;

use nestix::{Element, Shared, component};
use nestix_native_core::{
    ButtonProps, StyleContext, matched_style, resolve_font_props, resolved_view_style,
    style_appearance, style_padding_with_default,
};

use crate::{
    DomEventData, DomValue,
    renderer::{mount_host, renderer},
    style_declarations::{appearance_styles, font_styles, padding_styles, view_styles},
};

/// DOM push button.
#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__dom_Button"];

    let renderer = renderer(element);
    let node = renderer.create_element("button");
    mount_host(element, renderer.clone(), node);

    let on_click = props.on_click.clone();
    renderer.listen(
        node,
        "click".to_string(),
        Shared::from(Rc::new(move |_: &DomEventData| {
            if let Some(on_click) = on_click.get() {
                on_click();
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

            renderer.set_text(node, title.get());
            renderer.set_property(node, "disabled".to_string(), DomValue::Bool(disabled.get()));
            let scale_factor = renderer.scale_factor();
            let mut styles = view_styles(&style, scale_factor);
            styles.extend(padding_styles(&style, scale_factor));
            styles.extend(appearance_styles(style.appearance.unwrap_or_default()));
            styles.extend(font_styles(&resolve_font_props(
                Some(&style),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            )));
            renderer.replace_styles(node, styles);
        }
    });
}
