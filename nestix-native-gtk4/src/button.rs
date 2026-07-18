use gtk4::prelude::*;
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    Appearance, ButtonProps, Dimension, FontStyle, Rect, StyleContext, matched_style,
    resolve_font_props, style_appearance, style_padding_with_default,
};

use crate::{WindowContext, layout::mount_leaf};

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__gtk4_Button"];

    let window_context = element.context::<WindowContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let button = gtk4::Button::with_label(&props.title.get());
    let css = gtk4::CssProvider::new();
    button
        .style_context()
        .add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let content_revision = create_state(0usize);

    button.connect_clicked(closure!(
        [props.on_click] | _ | {
            if let Some(on_click) = on_click.get() {
                on_click();
            }
        }
    ));

    scoped_effect!(
        element,
        [button, props.disabled] || {
            button.set_sensitive(!disabled.get());
        }
    );
    scoped_effect!(
        element,
        [button, props.title, content_revision] || {
            button.set_label(&title.get());
            content_revision.mutate(|revision| *revision += 1);
        }
    );
    scoped_effect!(
        element,
        [
            css,
            style_props,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            props.container.padding(),
            props.appearance,
            window_context.scale_factor,
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
            let padding =
                style_padding_with_default(style_props.as_ref(), padding.get(), Dimension::Auto);
            let appearance = style_appearance(style_props.as_ref(), appearance.get());
            let native_appearance = uses_native_appearance(appearance, padding);
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
            if !native_appearance {
                declarations.extend([
                    "background-color: transparent".to_string(),
                    "background-image: none".to_string(),
                    "border-style: none".to_string(),
                    "box-shadow: none".to_string(),
                ]);
                let padding = logical_padding(padding, scale_factor.get());
                declarations.push(format!(
                    "padding: {}px {}px {}px {}px",
                    padding.top, padding.right, padding.bottom, padding.left,
                ));
            }

            let declarations = declarations.join("; ");
            let css_rule = if declarations.is_empty() {
                "button {}".to_string()
            } else {
                format!("button {{ {declarations}; }}")
            };
            css.load_from_data(&css_rule);
            content_revision.mutate(|revision| *revision += 1);
        }
    );

    mount_leaf(
        element,
        button.upcast_ref(),
        style_props,
        &props.view,
        content_revision.into_readonly(),
    );
}

fn uses_native_appearance(appearance: Appearance, padding: Rect<Dimension>) -> bool {
    match appearance {
        Appearance::Native => true,
        Appearance::None => false,
        Appearance::Auto => [padding.top, padding.right, padding.bottom, padding.left]
            .into_iter()
            .all(|dimension| dimension == Dimension::Auto),
    }
}

fn logical_padding(padding: Rect<Dimension>, scale_factor: f64) -> Rect<f32> {
    fn logical(dimension: Dimension, scale_factor: f64) -> f32 {
        match dimension {
            Dimension::Auto => 0.0,
            Dimension::Length(value) => value.to_logical::<f32>(scale_factor).into(),
        }
    }

    Rect {
        top: logical(padding.top, scale_factor),
        right: logical(padding.right, scale_factor),
        bottom: logical(padding.bottom, scale_factor),
        left: logical(padding.left, scale_factor),
    }
}

fn css_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_appearance_is_native_only_for_auto_padding() {
        let auto = Rect {
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
        };
        assert!(uses_native_appearance(Appearance::Auto, auto));

        let custom = Rect {
            left: Dimension::from(4),
            ..auto
        };
        assert!(!uses_native_appearance(Appearance::Auto, custom));
        assert!(uses_native_appearance(Appearance::Native, custom));
        assert!(!uses_native_appearance(Appearance::None, auto));
    }
}
