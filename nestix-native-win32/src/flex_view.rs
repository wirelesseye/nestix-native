use std::rc::Rc;

use crate::{WindowContext, contexts::ParentContext};
use nestix::{Element, closure, component, components::ContextProvider, layout, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, FlexViewProps, StyleContext, StyleScope, TreeContext, WithAuto, matched_style,
    resolved_flex_view_style, style_align_items, style_align_self, style_flex_basis,
    style_flex_direction, style_flex_grow, style_flex_shrink, style_flex_wrap, style_gap,
    style_justify_content, style_length_with_auto, style_margin, style_padding,
    utils::{gap_to_taffy, inset_to_taffy, margin_to_taffy, padding_to_taffy},
};
use taffy::{Size, Style};

#[component]
/// Lays out children in a virtual Win32 visual node using flexbox.
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    require_visual_mount!(element, FlexView, output);
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__win32_FlexView"];

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
    let effective_style = resolved_flex_view_style(matched_style_props.clone(), props);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        effective_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, effective_style, window_context.scale_factor] || {
            animated_style.set_target(effective_style.get(), scale_factor.get());
        }
    );
    let node_id = tree_context.create_node(false);
    let visual = parent_context.mount_virtual(node_id);
    element.provide_handle(visual.clone());
    element.on_place(closure!(
        [parent_context, visual] | placement | {
            parent_context.place_child(&visual, placement);
        }
    ));

    element.on_unmount(closure!(
        [parent_context, visual] || {
            parent_context.remove_child(&visual);
        }
    ));

    scoped_effect!(
        [visual, matched_style_props, props.bg_color] || {
            let style_props = matched_style_props.get();
            let bg_color = bg_color.get().or_else(|| {
                style_props
                    .as_ref()
                    .and_then(|style_props| style_props.bg_color)
            });
            visual.surface().set_background(visual.id(), bg_color);
        }
    );

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
            window_context,
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = window_context.scale_factor.get();
            let style_props = style_props.get();
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

            if parent_node.is_some() {
                // Update size when the node is not root
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
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
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.container.padding()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                padding: padding_to_taffy(
                    style_padding(style_props.as_ref(), padding.get()),
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
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.gap
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let gap = gap_to_taffy(style_gap(style_props.as_ref(), gap.get()), scale_factor);
            tree_context.update_style(node_id, |prev| Style {
                gap: Size {
                    width: gap,
                    height: gap,
                },
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.flex_direction] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: style_flex_direction(style_props.as_ref(), flex_direction.get())
                    .to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.align_items] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_items: style_align_items(style_props.as_ref(), align_items.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.justify_content] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                justify_content: style_justify_content(style_props.as_ref(), justify_content.get())
                    .to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.flex_wrap] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: style_flex_wrap(style_props.as_ref(), flex_wrap.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<ParentContext>(parent_context.child_context(&visual, node_id)) {
                $(props.children.clone())
            }
        }
    }
}
