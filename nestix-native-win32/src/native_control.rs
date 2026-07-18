use nestix::{Computed, Element, Readonly, closure, scoped_effect};
use nestix_native_core::{
    Dimension, ResolvedStyle, TreeContext, ViewProps,
    dpi::{LogicalPosition, LogicalSize},
    style_align_self, style_dimension, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{NodeId, Size, Style, prelude::FromLength};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DestroyWindow, SWP_NOZORDER, SetWindowPos},
};

use crate::{
    WindowContext,
    contexts::{ParentContext, native_predecessor},
};

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

    element.provide_handle(hwnd);
    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [parent_context] | _ | {
            if let Some(insert_child) = &parent_context.insert_child {
                insert_child(hwnd, Some(node_id), native_predecessor(&element));
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(hwnd, Some(node_id));
            }
        }
    ));
    element.on_unmount(closure!(
        [parent_context] || {
            unsafe { DestroyWindow(hwnd).unwrap() };
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
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
        element,
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
            let width =
                match style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width) {
                    Dimension::Auto => intrinsic.width,
                    Dimension::Length(value) => value.to_logical::<f32>(scale).0,
                };
            let height = match style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| {
                s.height
            }) {
                Dimension::Auto => intrinsic.height,
                Dimension::Length(value) => value.to_logical::<f32>(scale).0,
            };
            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width: taffy::Dimension::from_length(width),
                    height: taffy::Dimension::from_length(height),
                },
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
            props.left,
            props.top
        ] || {
            let style = style_props.get();
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
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
        element,
        [tree_context, style_props, props.align_self] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [window_context.scale_factor, tree_context] || {
            if let Some(layout) = tree_context.layout(node_id) {
                let point = LogicalPosition::new(layout.location.x, layout.location.y)
                    .to_physical(scale_factor.get());
                let size = LogicalSize::new(layout.size.width, layout.size.height)
                    .to_physical(scale_factor.get());
                unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        point.x,
                        point.y,
                        size.width,
                        size.height,
                        SWP_NOZORDER,
                    )
                    .unwrap();
                }
            }
        }
    );

    node_id
}
