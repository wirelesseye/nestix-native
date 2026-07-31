use std::rc::Rc;

use nestix::{Computed, Element, Readonly, closure, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, ResolvedStyle, TreeContext, ViewProps, WithAuto,
    dpi::LogicalSize,
    resolved_view_style, style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_length_with_auto, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{NodeId, Size, Style, prelude::FromLength};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::DestroyWindow};

use crate::{WindowContext, contexts::ParentContext};

pub(crate) fn mount(
    element: &Element,
    hwnd: HWND,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    intrinsic_size: Readonly<LogicalSize<f32>>,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let target_style = resolved_view_style(style_props, props);
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

    element.provide_handle(hwnd);
    let node_id = tree_context.create_node(true);
    let visual = parent_context.mount_native(node_id, hwnd);
    element.on_place(closure!(
        [parent_context, visual] | placement | {
            parent_context.place_child(&visual, placement);
        }
    ));
    element.on_unmount(closure!(
        [parent_context, visual] || {
            unsafe { DestroyWindow(hwnd).unwrap() };
            parent_context.remove_child(&visual);
        }
    ));

    scoped_effect!(
        [
            tree_context,
            style_props,
            props.flex_grow,
            props.flex_basis,
            props.flex_shrink,
            window_context.scale_factor
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style.as_ref(), flex_shrink.get()),
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
            props.width,
            props.height,
            intrinsic_size
        ] || {
            let scale = scale_factor.get();
            let style = style_props.get();
            let intrinsic = intrinsic_size.get();
            let (width, min_width) = leaf_dimension(
                style_length_with_auto(style.as_ref(), width.get(), WithAuto::Auto, |s| s.width),
                intrinsic.width,
                scale,
            );
            let (height, min_height) = leaf_dimension(
                style_length_with_auto(style.as_ref(), height.get(), WithAuto::Auto, |s| s.height),
                intrinsic.height,
                scale,
            );
            tree_context.update_style(node_id, |prev| Style {
                size: Size { width, height },
                min_size: Size {
                    width: min_width,
                    height: min_height,
                },
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
            props.position,
            props.left,
            props.top
        ] || {
            let style = style_props.get();
            let left =
                style_length_with_auto(style.as_ref(), left.get(), WithAuto::Auto, |s| s.left);
            let top = style_length_with_auto(style.as_ref(), top.get(), WithAuto::Auto, |s| s.top);
            tree_context.update_style(node_id, |prev| Style {
                position: nestix_native_core::style_position(style.as_ref(), position.get())
                    .to_taffy(),
                inset: inset_to_taffy(left, top, scale_factor.get()),
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
            props.margin()
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.align_self] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    node_id
}

pub(crate) fn leaf_dimension(
    value: WithAuto<nestix_native_core::Length>,
    intrinsic: f32,
    scale_factor: f64,
) -> (taffy::Dimension, taffy::Dimension) {
    match value {
        WithAuto::Auto => (
            taffy::Dimension::auto(),
            taffy::Dimension::from_length(intrinsic),
        ),
        WithAuto::Value(value) => (
            taffy::Dimension::from_length(value.to_logical::<f32>(scale_factor).0),
            taffy::Dimension::auto(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::{AlignItems, AvailableSpace, FlexDirection, TaffyTree};

    #[test]
    fn auto_native_leaf_width_can_stretch_past_intrinsic_size() {
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let leaf = tree
            .new_leaf(Style {
                size: Size {
                    width: taffy::Dimension::auto(),
                    height: taffy::Dimension::from_length(150.0),
                },
                min_size: Size {
                    width: taffy::Dimension::from_length(300.0),
                    height: taffy::Dimension::from_length(150.0),
                },
                ..Default::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    display: taffy::Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: Some(AlignItems::Stretch),
                    size: Size {
                        width: taffy::Dimension::from_length(600.0),
                        height: taffy::Dimension::from_length(400.0),
                    },
                    ..Default::default()
                },
                &[leaf],
            )
            .unwrap();

        tree.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(600.0),
                height: AvailableSpace::Definite(400.0),
            },
        )
        .unwrap();

        assert_eq!(tree.layout(leaf).unwrap().size.width, 600.0);
    }
}
