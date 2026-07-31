use gtk4::{Orientation, Widget, prelude::*};
use nestix::{Element, Readonly, closure, scoped_effect};
use nestix_native_core::{
    ResolvedStyle, TreeContext, ViewProps, WithAuto,
    dpi::LogicalSize,
    style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink, style_length_with_auto,
    style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{LayoutRefreshContext, ParentContext},
};

pub(crate) fn mount_leaf(
    element: &Element,
    widget: &Widget,
    style_props: Readonly<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    mount_leaf_inner(
        element,
        widget,
        style_props,
        props,
        content_revision,
        true,
        None,
    )
}

pub(crate) fn mount_leaf_with_stretchable_width(
    element: &Element,
    widget: &Widget,
    style_props: Readonly<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    mount_leaf_inner(
        element,
        widget,
        style_props,
        props,
        content_revision,
        false,
        None,
    )
}

pub(crate) fn mount_leaf_with_intrinsic_size(
    element: &Element,
    widget: &Widget,
    style_props: Readonly<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
    intrinsic_size: LogicalSize<f32>,
) -> NodeId {
    mount_leaf_inner(
        element,
        widget,
        style_props,
        props,
        content_revision,
        true,
        Some(intrinsic_size),
    )
}

fn mount_leaf_inner(
    element: &Element,
    widget: &Widget,
    style_props: Readonly<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
    intrinsic_auto_width: bool,
    intrinsic_size: Option<LogicalSize<f32>>,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let layout_refresh = element.context::<LayoutRefreshContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let widget = widget.clone();
    let node_id = tree_context.create_node(true);
    element.provide_handle(widget.clone());

    element.on_place(closure!(
        [widget, parent_context] | placement | {
            parent_context.place_child(&widget, Some(node_id), placement);
        }
    ));
    element.on_unmount(closure!(
        [widget, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&widget, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.flex_grow,
            props.flex_basis,
            props.flex_shrink,
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
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            widget,
            props.width,
            props.height,
            content_revision,
            window_context.scale_factor
        ] || {
            let _ = content_revision.get();
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
            let (_, natural_width, _, _) = widget.measure(Orientation::Horizontal, -1);
            let (_, natural_height, _, _) = widget.measure(Orientation::Vertical, natural_width);
            let (width, min_width) = intrinsic_dimension(
                width,
                intrinsic_size.map(|size| size.width),
                natural_width,
                intrinsic_auto_width,
                scale_factor.get(),
            );
            let (height, min_height) = intrinsic_dimension(
                height,
                intrinsic_size.map(|size| size.height),
                natural_height,
                true,
                scale_factor.get(),
            );
            tree_context.update_style(node_id, |prev| Style {
                size: Size { width, height },
                min_size: Size {
                    width: min_width,
                    height: min_height,
                },
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.position,
            props.left,
            props.top,
            window_context.scale_factor
        ] || {
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
                position: nestix_native_core::style_position(style_props.as_ref(), position.get())
                    .to_taffy(),
                inset: inset_to_taffy(left, top, scale_factor.get()),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );
    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.margin(),
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );
    scoped_effect!(
        [tree_context, layout_refresh, style_props, props.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );
    scoped_effect!(
        [
            tree_context,
            parent_context.parent_node,
            parent_context.fixed,
            widget
        ] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                widget.set_size_request(
                    layout.size.width.round() as i32,
                    layout.size.height.round() as i32,
                );
                if let Some(fixed) = &fixed
                    && widget.parent().as_ref() == Some(fixed.upcast_ref())
                {
                    fixed.move_(&widget, layout.location.x as f64, layout.location.y as f64);
                    fixed.queue_allocate();
                }
            }
        }
    );
    node_id
}

fn intrinsic_dimension(
    value: WithAuto<nestix_native_core::Length>,
    fallback: Option<f32>,
    measured: i32,
    intrinsic_auto: bool,
    scale_factor: f64,
) -> (taffy::Dimension, taffy::Dimension) {
    match value {
        WithAuto::Auto => match fallback {
            Some(fallback) => (
                taffy::Dimension::auto(),
                taffy::Dimension::from_length(fallback),
            ),
            None if !intrinsic_auto => (taffy::Dimension::auto(), taffy::Dimension::auto()),
            None => (
                taffy::Dimension::from_length(measured as f32),
                taffy::Dimension::auto(),
            ),
        },
        WithAuto::Value(value) => (
            taffy::Dimension::from_length(value.to_logical::<f32>(scale_factor)),
            taffy::Dimension::auto(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::{FlexDirection, TaffyTree};

    #[test]
    fn intrinsic_fallback_keeps_auto_size_stretchable() {
        let (size, minimum) = intrinsic_dimension(WithAuto::Auto, Some(300.0), 0, true, 1.0);
        assert_eq!(size, taffy::Dimension::auto());
        assert_eq!(minimum, taffy::Dimension::from_length(300.0_f32));
    }

    #[test]
    fn intrinsic_fallback_allows_a_leaf_to_grow_and_stretch() {
        let mut tree = TaffyTree::<()>::new();
        let leaf = tree
            .new_leaf(Style {
                flex_grow: 1.0,
                size: Size::auto(),
                min_size: Size {
                    width: taffy::Dimension::from_length(300.0_f32),
                    height: taffy::Dimension::from_length(150.0_f32),
                },
                ..Style::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    size: Size {
                        width: taffy::Dimension::from_length(900.0_f32),
                        height: taffy::Dimension::from_length(650.0_f32),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                },
                &[leaf],
            )
            .unwrap();

        tree.compute_layout(root, Size::max_content()).unwrap();
        assert_eq!(
            tree.layout(leaf).unwrap().size,
            Size {
                width: 900.0,
                height: 650.0,
            }
        );
    }
}
