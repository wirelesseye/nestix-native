use std::rc::Rc;

use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, StyleContext, TextProps, TreeContext, WithAuto, matched_style,
    resolve_font_props, resolved_view_style, style_align_self, style_flex_basis, style_flex_grow,
    style_flex_shrink, style_length_with_auto, style_margin,
};
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::ParentContext,
    font::{ns_color, resolve_font},
};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    require_visual_mount!(element, Text);
    const DEFAULT_CLASSES: [&str; 2] = ["__Text", "__appkit_Text"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let matched_style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_style = resolved_view_style(matched_style_props, &props.view);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        target_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, target_style, window_context.scale_factor] || {
            animated_style.set_target(target_style.get(), scale_factor.get());
        }
    );

    let mtm = MainThreadMarker::new().unwrap();
    let ns_string = NSString::from_str(&props.text.get());
    let label = NSTextField::labelWithString(&ns_string, mtm);
    let original_font = label.font().unwrap();
    let original_color = label.textColor();
    element.provide_handle(label.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [label, parent_context] | placement | {
            parent_context.place_child(&label, Some(node_id), placement);
        }
    ));

    element.on_unmount(closure!(
        [parent_context, label] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&label, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            parent_context.parent_node,
            tree_context,
            style_props,
            label,
            props.view.width,
            props.view.height,
            props.text,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            original_font,
            original_color,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let ns_string = NSString::from_str(&text.get());
            label.setStringValue(&ns_string);
            let font_props = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let font = resolve_font(&original_font, &font_props, mtm);
            label.setFont(Some(&font));
            if let Some(color) = font_props.text_color {
                label.setTextColor(Some(&ns_color(color)));
            } else {
                label.setTextColor(original_color.as_deref());
            }
            let width = style_length_with_auto(
                style_props.as_ref(),
                width.get(),
                WithAuto::Auto,
                |style| style.width,
            );
            let height = style_length_with_auto(
                style_props.as_ref(),
                height.get(),
                WithAuto::Auto,
                |style| style.height,
            );

            let intrinsic_size =
                (width.is_auto() || height.is_auto()).then(|| label.intrinsicContentSize());
            let width = match width {
                WithAuto::Auto => intrinsic_size.unwrap().width as f32,
                WithAuto::Value(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height {
                WithAuto::Auto => intrinsic_size.unwrap().height as f32,
                WithAuto::Value(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_length_with_auto(style_props.as_ref(), left.get(), WithAuto::Auto, |style| {
                    style.left
                });
            let top =
                style_length_with_auto(style_props.as_ref(), top.get(), WithAuto::Auto, |style| {
                    style.top
                });

            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, parent_context.parent_node, label] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let alignment_rect = NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                );
                label.setFrame(label.frameForAlignmentRect(alignment_rect));
            }
        }
    );
}
