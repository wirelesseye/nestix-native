use nestix_native_core::{
    AlignItems, Appearance, Color, Easing, FlexDirection, FlexWrap, FontStyle, JustifyContent,
    Length, Position, ResolvedFontProps, ResolvedStyle, TransitionProperty, WithAuto,
};

use crate::DomStyle;

pub(crate) fn view_styles(style: &ResolvedStyle, scale_factor: f64) -> Vec<DomStyle> {
    let mut styles = vec![
        DomStyle::new("box-sizing", "border-box"),
        DomStyle::new(
            "position",
            match style.position.unwrap_or_default() {
                Position::Relative => "relative",
                Position::Absolute => "absolute",
            },
        ),
    ];
    push_length(&mut styles, "left", style.left, scale_factor);
    push_length(&mut styles, "top", style.top, scale_factor);
    push_length(&mut styles, "width", style.width, scale_factor);
    push_length(&mut styles, "height", style.height, scale_factor);
    push_length(&mut styles, "margin-left", style.margin_left, scale_factor);
    push_length(
        &mut styles,
        "margin-right",
        style.margin_right,
        scale_factor,
    );
    push_length(&mut styles, "margin-top", style.margin_top, scale_factor);
    push_length(
        &mut styles,
        "margin-bottom",
        style.margin_bottom,
        scale_factor,
    );
    push_optional(
        &mut styles,
        "flex-grow",
        style.flex_grow.map(|value| value.to_string()),
    );
    push_length(&mut styles, "flex-basis", style.flex_basis, scale_factor);
    push_optional(
        &mut styles,
        "flex-shrink",
        style.flex_shrink.map(|value| value.to_string()),
    );
    push_optional(
        &mut styles,
        "align-self",
        style.align_self.map(align_items).map(str::to_string),
    );
    push_optional(&mut styles, "background-color", style.bg_color.map(color));
    styles.extend(font_styles(&style.font()));
    let transitions = transitions(style);
    if !transitions.is_empty() {
        styles.push(DomStyle::new("transition", transitions));
    }
    styles
}

pub(crate) fn flex_styles(style: &ResolvedStyle, scale_factor: f64) -> Vec<DomStyle> {
    let mut styles = view_styles(style, scale_factor);
    styles.push(DomStyle::new("display", "flex"));
    push_optional(
        &mut styles,
        "flex-direction",
        style
            .flex_direction
            .map(|value| match value {
                FlexDirection::Row => "row",
                FlexDirection::RowReverse => "row-reverse",
                FlexDirection::Column => "column",
                FlexDirection::ColumnReverse => "column-reverse",
            })
            .map(str::to_string),
    );
    push_optional(
        &mut styles,
        "align-items",
        style.align_items.map(align_items).map(str::to_string),
    );
    push_optional(
        &mut styles,
        "justify-content",
        style
            .justify_content
            .map(|value| match value {
                JustifyContent::Normal => "normal",
                JustifyContent::Start => "start",
                JustifyContent::End => "end",
                JustifyContent::FlexStart => "flex-start",
                JustifyContent::FlexEnd => "flex-end",
                JustifyContent::Center => "center",
                JustifyContent::Stretch => "stretch",
                JustifyContent::SpaceBetween => "space-between",
                JustifyContent::SpaceEvenly => "space-evenly",
                JustifyContent::SpaceAround => "space-around",
            })
            .map(str::to_string),
    );
    push_optional(
        &mut styles,
        "flex-wrap",
        style
            .flex_wrap
            .map(|value| match value {
                FlexWrap::NoWrap => "nowrap",
                FlexWrap::Wrap => "wrap",
            })
            .map(str::to_string),
    );
    push_length(&mut styles, "gap", style.gap, scale_factor);
    styles.extend(padding_styles(style, scale_factor));
    styles
}

pub(crate) fn padding_styles(style: &ResolvedStyle, scale_factor: f64) -> Vec<DomStyle> {
    let mut styles = Vec::new();
    push_length(
        &mut styles,
        "padding-left",
        style.padding_left,
        scale_factor,
    );
    push_length(
        &mut styles,
        "padding-right",
        style.padding_right,
        scale_factor,
    );
    push_length(&mut styles, "padding-top", style.padding_top, scale_factor);
    push_length(
        &mut styles,
        "padding-bottom",
        style.padding_bottom,
        scale_factor,
    );
    styles
}

pub(crate) fn font_styles(font: &ResolvedFontProps) -> Vec<DomStyle> {
    let mut styles = Vec::new();
    push_optional(&mut styles, "font-family", font.font_family.clone());
    push_optional(
        &mut styles,
        "font-size",
        font.font_size.map(|value| format!("{value}px")),
    );
    push_optional(
        &mut styles,
        "font-weight",
        font.font_weight.map(|value| value.value().to_string()),
    );
    push_optional(
        &mut styles,
        "font-style",
        font.font_style.map(|value| match value {
            FontStyle::Normal => "normal".to_string(),
            FontStyle::Italic => "italic".to_string(),
        }),
    );
    push_optional(&mut styles, "color", font.text_color.map(color));
    styles
}

pub(crate) fn appearance_styles(appearance: Appearance) -> Vec<DomStyle> {
    match appearance {
        Appearance::None => vec![DomStyle::new("appearance", "none")],
        Appearance::Native | Appearance::Auto => Vec::new(),
    }
}

fn push_optional(styles: &mut Vec<DomStyle>, property: &str, value: Option<String>) {
    if let Some(value) = value {
        styles.push(DomStyle::new(property, value));
    }
}

fn push_length(
    styles: &mut Vec<DomStyle>,
    property: &str,
    value: Option<WithAuto<Length>>,
    scale_factor: f64,
) {
    push_optional(
        styles,
        property,
        value.map(|value| length(value, scale_factor)),
    );
}

fn length(value: WithAuto<Length>, scale_factor: f64) -> String {
    match value {
        WithAuto::Auto => "auto".to_string(),
        WithAuto::Value(Length::Logical(value)) => format!("{value}px"),
        WithAuto::Value(Length::Physical(value)) => {
            format!("{}px", f64::from(value) / scale_factor)
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

fn transitions(style: &ResolvedStyle) -> String {
    style
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
        .join(", ")
}
