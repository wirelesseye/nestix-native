use std::rc::Rc;

use nestix::{Element, Layout, Shared, component, components::Fragment, layout, props};
use nestix_native_core::{
    AlignItems, Appearance, ButtonProps, Color, Easing, FlexDirection, FlexViewProps, FlexWrap,
    FontStyle, InputProps, JustifyContent, Length, ResolvedFontProps, ResolvedStyle, StyleContext,
    StyleScope, TextProps, TransitionProperty, WithAuto, matched_style, resolve_font_props,
    resolved_flex_view_style, resolved_view_style, style_appearance, style_padding_with_default,
};

use crate::{
    DomEventData, DomNodeHandle, DomRuntimeContext, DomStyle, DomValue, EmbeddedDomRuntime,
};

/// Root node of one embedded managed DOM document.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DomDocumentRootProps {
    #[props(default)]
    pub children: Layout,
}

#[component]
pub fn DomDocumentRoot(props: &DomDocumentRootProps, element: &Element) -> Element {
    let runtime = element
        .context::<DomRuntimeContext>()
        .expect("DOM components must be mounted beneath DomSurface")
        .runtime
        .clone();
    element.provide_handle(runtime.root_handle());
    layout! { Fragment(.children = props.children.clone()) }
}

/// DOM push button rendered through an embedded command transport.
#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__dom_Button"];

    let runtime = runtime(element);
    let node = runtime.create_element("button");
    mount_host(element, runtime.clone(), node);

    let on_click = props.on_click.clone();
    runtime.listen(
        node,
        "click",
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
        let runtime = runtime.clone();
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

            runtime.set_text(node, title.get());
            runtime.set_property(node, "disabled", DomValue::Bool(disabled.get()));
            let mut styles = view_styles(&style);
            styles.extend(padding_styles(&style));
            styles.extend(appearance_styles(style.appearance.unwrap_or_default()));
            styles.extend(font_styles(&resolve_font_props(
                Some(&style),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            )));
            runtime.replace_styles(node, styles);
        }
    });
}

/// DOM single-line input rendered through an embedded command transport.
#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__dom_Input"];

    let runtime = runtime(element);
    let node = runtime.create_element("input");
    runtime.set_attribute(node, "type", Some("text".to_string()));
    mount_host(element, runtime.clone(), node);

    let on_text_change = props.on_text_change.clone();
    runtime.listen(
        node,
        "input",
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
        let runtime = runtime.clone();
        let effective_style = effective_style.clone();
        let value = props.value.clone();
        move || {
            runtime.set_property(node, "value", value.get());
            runtime.replace_styles(
                node,
                view_styles(&effective_style.get().unwrap_or_default()),
            );
        }
    });
}

/// DOM text label rendered through an embedded command transport.
#[component]
pub fn Text(props: &TextProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__dom_Text"];

    let runtime = runtime(element);
    let node = runtime.create_element("span");
    mount_host(element, runtime.clone(), node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let runtime = runtime.clone();
        let effective_style = effective_style.clone();
        let text = props.text.clone();
        let font_family = props.font.font_family.clone();
        let font_size = props.font.font_size.clone();
        let font_weight = props.font.font_weight.clone();
        let font_style = props.font.font_style.clone();
        let text_color = props.font.text_color.clone();
        move || {
            let style = effective_style.get().unwrap_or_default();
            let mut styles = view_styles(&style);
            styles.extend(font_styles(&resolve_font_props(
                Some(&style),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            )));
            runtime.set_text(node, text.get());
            runtime.replace_styles(node, styles);
        }
    });
}

/// DOM flex container rendered through an embedded command transport.
#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__dom_FlexView"];

    let runtime = runtime(element);
    let node = runtime.create_element("div");
    mount_host(element, runtime.clone(), node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(matched, props);
    element.scoped_effect({
        let runtime = runtime.clone();
        let effective_style = effective_style.clone();
        move || {
            runtime.replace_styles(
                node,
                flex_styles(&effective_style.get().unwrap_or_default()),
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

fn runtime(element: &Element) -> Rc<EmbeddedDomRuntime> {
    element
        .context::<DomRuntimeContext>()
        .expect("DOM components must be mounted beneath DomSurface")
        .runtime
        .clone()
}

fn mount_host(element: &Element, runtime: Rc<EmbeddedDomRuntime>, node: DomNodeHandle) {
    element.provide_handle(node);
    element.on_place({
        let runtime = runtime.clone();
        move |placement| {
            let parent = placement
                .parent
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<DomNodeHandle>())
                .copied();
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<DomNodeHandle>())
                .copied();
            if let Some(parent) = parent {
                runtime.place(node, parent, predecessor);
            }
        }
    });
    element.on_unmount(move || runtime.remove(node));
}

fn view_styles(style: &ResolvedStyle) -> Vec<DomStyle> {
    let mut styles = vec![
        DomStyle::new("box-sizing", "border-box"),
        DomStyle::new("position", "relative"),
    ];
    push_length(&mut styles, "left", style.left);
    push_length(&mut styles, "top", style.top);
    push_length(&mut styles, "width", style.width);
    push_length(&mut styles, "height", style.height);
    push_length(&mut styles, "margin-left", style.margin_left);
    push_length(&mut styles, "margin-right", style.margin_right);
    push_length(&mut styles, "margin-top", style.margin_top);
    push_length(&mut styles, "margin-bottom", style.margin_bottom);
    push_optional(
        &mut styles,
        "flex-grow",
        style.flex_grow.map(|v| v.to_string()),
    );
    push_length(&mut styles, "flex-basis", style.flex_basis);
    push_optional(
        &mut styles,
        "flex-shrink",
        style.flex_shrink.map(|v| v.to_string()),
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

fn flex_styles(style: &ResolvedStyle) -> Vec<DomStyle> {
    let mut styles = view_styles(style);
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
    push_length(&mut styles, "gap", style.gap);
    styles.extend(padding_styles(style));
    styles
}

fn padding_styles(style: &ResolvedStyle) -> Vec<DomStyle> {
    let mut styles = Vec::new();
    push_length(&mut styles, "padding-left", style.padding_left);
    push_length(&mut styles, "padding-right", style.padding_right);
    push_length(&mut styles, "padding-top", style.padding_top);
    push_length(&mut styles, "padding-bottom", style.padding_bottom);
    styles
}

fn font_styles(font: &ResolvedFontProps) -> Vec<DomStyle> {
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

fn appearance_styles(appearance: Appearance) -> Vec<DomStyle> {
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

fn push_length(styles: &mut Vec<DomStyle>, property: &str, value: Option<WithAuto<Length>>) {
    push_optional(styles, property, value.map(length));
}

fn length(value: WithAuto<Length>) -> String {
    match value {
        WithAuto::Auto => "auto".to_string(),
        WithAuto::Value(Length::Logical(value)) => format!("{value}px"),
        WithAuto::Value(Length::Physical(value)) => format!("{value}px"),
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
