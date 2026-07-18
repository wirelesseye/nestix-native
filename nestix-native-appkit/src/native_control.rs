use nestix::{Computed, Element, Readonly, closure, scoped_effect};
use nestix_native_core::{
    Dimension, ResolvedStyle, TreeContext, ViewProps, style_align_self, style_dimension,
    style_flex_basis, style_flex_grow, style_flex_shrink, style_margin,
};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{ParentContext, native_predecessor},
};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

pub(crate) fn mount(
    element: &Element,
    view: Retained<NSView>,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    element.provide_handle(view.as_ref() as *const NSView as *const NSObject);
    let node_id = tree_context.create_node(true);

    element.on_place(closure!(
        [element, view, parent_context] | _ | {
            if let Some(insert_child) = &parent_context.insert_child {
                insert_child(&view, Some(node_id), native_predecessor(&element));
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&view, Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [view, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&view, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [
            tree_context,
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
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            style_props,
            view,
            props.width,
            props.height,
            content_revision
        ] || {
            let _ = content_revision.get();
            let scale_factor = scale_factor.get();
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
            let intrinsic_size =
                (width.is_auto() || height.is_auto()).then(|| view.intrinsicContentSize());
            let width = match width {
                Dimension::Auto => intrinsic_size.unwrap().width as f32,
                Dimension::Length(value) => value.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height {
                Dimension::Auto => intrinsic_size.unwrap().height as f32,
                Dimension::Length(value) => value.to_logical::<f32>(scale_factor).into(),
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
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.left,
            props.top
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
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.margin()
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, view] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let alignment_rect = NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                );
                view.setFrame(view.frameForAlignmentRect(alignment_rect));
            }
        }
    );

    node_id
}
