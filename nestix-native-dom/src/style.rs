use nestix_native_core::{
    AlignItems, Color, Easing, FontStyle, Length, ResolvedFontProps, ResolvedStyle,
    TransitionProperty, WithAuto,
};
use web_sys::CssStyleDeclaration;

pub(crate) fn apply_view_style(css: &CssStyleDeclaration, style: &ResolvedStyle) {
    set(css, "box-sizing", "border-box");
    set(css, "position", "relative");
    set_length(css, "left", style.left);
    set_length(css, "top", style.top);
    set_length(css, "width", style.width);
    set_length(css, "height", style.height);
    set_length(css, "margin-left", style.margin_left);
    set_length(css, "margin-right", style.margin_right);
    set_length(css, "margin-top", style.margin_top);
    set_length(css, "margin-bottom", style.margin_bottom);
    set_optional(
        css,
        "flex-grow",
        style.flex_grow.map(|value| value.to_string()),
    );
    set_length(css, "flex-basis", style.flex_basis);
    set_optional(
        css,
        "flex-shrink",
        style.flex_shrink.map(|value| value.to_string()),
    );
    set_optional(css, "align-self", style.align_self.map(align_items));
    apply_font(css, &style.font());
    set_optional(css, "background-color", style.bg_color.map(color));
    apply_transitions(css, style);
}

pub(crate) fn apply_padding(css: &CssStyleDeclaration, style: &ResolvedStyle) {
    set_length(css, "padding-left", style.padding_left);
    set_length(css, "padding-right", style.padding_right);
    set_length(css, "padding-top", style.padding_top);
    set_length(css, "padding-bottom", style.padding_bottom);
}

pub(crate) fn apply_font(css: &CssStyleDeclaration, font: &ResolvedFontProps) {
    set_optional(css, "font-family", font.font_family.clone());
    set_optional(
        css,
        "font-size",
        font.font_size.map(|value| format!("{value}px")),
    );
    set_optional(
        css,
        "font-weight",
        font.font_weight.map(|value| value.value().to_string()),
    );
    set_optional(
        css,
        "font-style",
        font.font_style.map(|value| match value {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
        }),
    );
    set_optional(css, "color", font.text_color.map(color));
}

pub(crate) fn set_length(
    css: &CssStyleDeclaration,
    property: &str,
    value: Option<WithAuto<Length>>,
) {
    set_optional(css, property, value.map(length_with_auto));
}

pub(crate) fn set(css: &CssStyleDeclaration, property: &str, value: &str) {
    css.set_property(property, value)
        .unwrap_or_else(|_| panic!("failed to set CSS property `{property}`"));
}

pub(crate) fn remove(css: &CssStyleDeclaration, property: &str) {
    css.remove_property(property)
        .unwrap_or_else(|_| panic!("failed to remove CSS property `{property}`"));
}

fn set_optional(css: &CssStyleDeclaration, property: &str, value: Option<impl AsRef<str>>) {
    if let Some(value) = value {
        set(css, property, value.as_ref());
    } else {
        remove(css, property);
    }
}

fn length_with_auto(value: WithAuto<Length>) -> String {
    match value {
        WithAuto::Auto => "auto".to_string(),
        WithAuto::Value(Length::Logical(value)) => format!("{value}px"),
        WithAuto::Value(Length::Physical(value)) => {
            let ratio = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
            format!("{}px", f64::from(value) / ratio)
        }
        WithAuto::Value(Length::Em(value)) => format!("{value}em"),
    }
}

fn color(value: Color) -> String {
    let value = value.into_rgb();
    format!(
        "rgba({}, {}, {}, {})",
        value.red,
        value.green,
        value.blue,
        f64::from(value.alpha) / 255.0
    )
}

fn align_items(value: AlignItems) -> &'static str {
    match value {
        AlignItems::Normal => "normal",
        AlignItems::Start => "start",
        AlignItems::End => "end",
        AlignItems::FlexStart => "flex-start",
        AlignItems::FlexEnd => "flex-end",
        AlignItems::Center => "center",
        AlignItems::Baseline => "baseline",
        AlignItems::Stretch => "stretch",
    }
}

fn apply_transitions(css: &CssStyleDeclaration, style: &ResolvedStyle) {
    let transitions = style
        .transitions
        .iter()
        .map(|transition| {
            let property = match transition.property {
                TransitionProperty::Left => "left",
                TransitionProperty::Top => "top",
                TransitionProperty::Width => "width",
                TransitionProperty::Height => "height",
                TransitionProperty::MarginLeft => "margin-left",
                TransitionProperty::MarginRight => "margin-right",
                TransitionProperty::MarginTop => "margin-top",
                TransitionProperty::MarginBottom => "margin-bottom",
                TransitionProperty::PaddingLeft => "padding-left",
                TransitionProperty::PaddingRight => "padding-right",
                TransitionProperty::PaddingTop => "padding-top",
                TransitionProperty::PaddingBottom => "padding-bottom",
                TransitionProperty::Gap => "gap",
            };
            let easing = match transition.animation.easing {
                Easing::Linear => "linear",
                Easing::EaseIn => "ease-in",
                Easing::EaseOut => "ease-out",
                Easing::EaseInOut => "ease-in-out",
            };
            format!(
                "{property} {}ms {easing}",
                transition.animation.duration.as_secs_f64() * 1000.0
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    if transitions.is_empty() {
        remove(css, "transition");
    } else {
        set(css, "transition", &transitions);
    }
}
