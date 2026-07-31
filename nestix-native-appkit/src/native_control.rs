use nestix::{Computed, Element, Readonly, closure, scoped_effect};
use nestix_native_core::dpi::LogicalSize;
use nestix_native_core::{
    ResolvedStyle, TreeContext, ViewProps, WithAuto, style_align_self, style_flex_basis,
    style_flex_grow, style_flex_shrink, style_length_with_auto, style_margin,
};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

pub(crate) fn mount(
    element: &Element,
    view: Retained<NSView>,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
) -> NodeId {
    mount_inner(element, view, style_props, props, content_revision, None)
}

pub(crate) fn mount_with_intrinsic_size(
    element: &Element,
    view: Retained<NSView>,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
    intrinsic_size: LogicalSize<f32>,
) -> NodeId {
    mount_inner(
        element,
        view,
        style_props,
        props,
        content_revision,
        Some(intrinsic_size),
    )
}

fn mount_inner(
    element: &Element,
    view: Retained<NSView>,
    style_props: Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
    content_revision: Readonly<usize>,
    intrinsic_size: Option<LogicalSize<f32>>,
) -> NodeId {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    element.provide_handle(view.as_ref() as *const NSView as *const NSObject);
    let node_id = tree_context.create_node(true);

    element.on_place(closure!(
        [view, parent_context] | placement | {
            parent_context.place_child(&view, Some(node_id), placement);
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
            let measured_size =
                (width.is_auto() || height.is_auto()).then(|| view.intrinsicContentSize());
            let (width, min_width) = intrinsic_dimension(
                width,
                intrinsic_size.map(|size| size.width),
                measured_size.map(|size| size.width as f32),
                scale_factor,
            );
            let (height, min_height) = intrinsic_dimension(
                height,
                intrinsic_size.map(|size| size.height),
                measured_size.map(|size| size.height as f32),
                scale_factor,
            );

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size { width, height },
                    min_size: Size {
                        width: min_width,
                        height: min_height,
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
            props.position,
            props.left,
            props.top
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

fn intrinsic_dimension(
    value: WithAuto<nestix_native_core::Length>,
    fallback: Option<f32>,
    measured: Option<f32>,
    scale_factor: f64,
) -> (taffy::Dimension, taffy::Dimension) {
    match value {
        WithAuto::Auto => match fallback {
            Some(fallback) => (
                taffy::Dimension::auto(),
                taffy::Dimension::from_length(fallback),
            ),
            None => (
                taffy::Dimension::from_length(measured.unwrap()),
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
    use taffy::{FlexDirection, Size, Style, TaffyTree, prelude::FromLength};

    #[test]
    fn intrinsic_minimum_allows_a_web_view_to_grow_and_stretch() {
        let mut tree = TaffyTree::<()>::new();
        let heading = tree
            .new_leaf(Style {
                size: Size {
                    width: taffy::Dimension::auto(),
                    height: taffy::Dimension::from_length(30.0),
                },
                ..Style::default()
            })
            .unwrap();
        let toolbar = tree
            .new_leaf(Style {
                size: Size {
                    width: taffy::Dimension::auto(),
                    height: taffy::Dimension::from_length(40.0),
                },
                ..Style::default()
            })
            .unwrap();
        let web_view = tree
            .new_leaf(Style {
                flex_grow: 1.0,
                size: Size::auto(),
                min_size: Size {
                    width: taffy::Dimension::from_length(300.0),
                    height: taffy::Dimension::from_length(150.0),
                },
                ..Style::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    size: Size {
                        width: taffy::Dimension::from_length(900.0),
                        height: taffy::Dimension::from_length(650.0),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                },
                &[heading, toolbar, web_view],
            )
            .unwrap();

        tree.compute_layout(root, Size::max_content()).unwrap();
        let layout = tree.layout(web_view).unwrap();
        assert_eq!(layout.size.width, 900.0);
        assert_eq!(layout.size.height, 580.0);
    }
}
