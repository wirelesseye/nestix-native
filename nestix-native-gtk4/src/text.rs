use std::{cell::RefCell, rc::Rc};

use gtk4::prelude::*;
use nestix::{Element, component, create_state, scoped_effect};
use nestix_native_core::{FontStyle, StyleContext, TextProps, matched_style, resolve_font_props};

use crate::layout::mount_leaf;

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__gtk4_Text"];

    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let label = gtk4::Label::new(Some(&props.text.get()));
    label.set_xalign(0.0);
    let css = gtk4::CssProvider::new();
    label
        .style_context()
        .add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let content_revision = create_state(0usize);
    let last_css = Rc::new(RefCell::new(None::<String>));

    scoped_effect!(
        [label, props.text, content_revision] || {
            label.set_text(&text.get());
            content_revision.mutate(|revision| *revision += 1);
        }
    );
    scoped_effect!(
        [
            css,
            last_css,
            style_props,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            content_revision
        ] || {
            let style_props = style_props.get();
            let font = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let mut declarations = Vec::new();
            if let Some(family) = font.font_family {
                declarations.push(format!("font-family: \"{}\"", css_string(&family)));
            }
            if let Some(size) = font.font_size {
                declarations.push(format!("font-size: {size}px"));
            }
            if let Some(weight) = font.font_weight {
                declarations.push(format!("font-weight: {}", weight.value()));
            }
            if let Some(style) = font.font_style {
                declarations.push(format!(
                    "font-style: {}",
                    match style {
                        FontStyle::Normal => "normal",
                        FontStyle::Italic => "italic",
                    }
                ));
            }
            if let Some(color) = font.text_color {
                let color = color.into_rgb();
                declarations.push(format!(
                    "color: rgba({}, {}, {}, {:.3})",
                    color.red,
                    color.green,
                    color.blue,
                    color.alpha as f64 / 255.0,
                ));
            }
            let declarations = declarations.join("; ");
            let css_rule = if declarations.is_empty() {
                "label {}".to_string()
            } else {
                format!("label {{ {declarations}; }}")
            };
            if last_css.borrow().as_ref() == Some(&css_rule) {
                return;
            }
            css.load_from_data(&css_rule);
            last_css.replace(Some(css_rule));
            content_revision.mutate(|revision| *revision += 1);
        }
    );

    mount_leaf(
        element,
        label.upcast_ref(),
        style_props,
        &props.view,
        content_revision.into_readonly(),
    );
}

fn css_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}
