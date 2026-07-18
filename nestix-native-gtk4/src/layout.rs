use gtk4::{Orientation, Widget, prelude::*};
use nestix::{Computed, Element, Readonly, closure, scoped_effect};
use nestix_native_core::{
    Dimension, ResolvedStyle, TreeContext, ViewProps, style_align_self, style_dimension,
    style_flex_basis, style_flex_grow, style_flex_shrink, style_margin,
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
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    mount_leaf_inner(element, widget, style_props, props, content_revision, true)
}

pub(crate) fn mount_leaf_with_stretchable_width(
    element: &Element,
    widget: &Widget,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    mount_leaf_inner(element, widget, style_props, props, content_revision, false)
}

fn mount_leaf_inner(
    element: &Element,
    widget: &Widget,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
    intrinsic_auto_width: bool,
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
        element,
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
        element,
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
            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            );
            let (_, natural_width, _, _) = widget.measure(Orientation::Horizontal, -1);
            let (_, natural_height, _, _) = widget.measure(Orientation::Vertical, natural_width);
            let width = match width {
                Dimension::Auto if !intrinsic_auto_width => taffy::Dimension::auto(),
                Dimension::Auto => taffy::Dimension::from_length(natural_width as f32),
                Dimension::Length(value) => {
                    taffy::Dimension::from_length(value.to_logical::<f32>(scale_factor.get()))
                }
            };
            let height = match height {
                Dimension::Auto => natural_height as f32,
                Dimension::Length(value) => value.to_logical::<f32>(scale_factor.get()).into(),
            };
            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width,
                    height: taffy::Dimension::from_length(height),
                },
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            layout_refresh,
            style_props,
            props.left,
            props.top,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
                style.top
            });
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor.get()),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );
    scoped_effect!(
        element,
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
        element,
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
        element,
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
