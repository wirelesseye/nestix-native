use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    ButtonProps, Dimension, StyleContext, TreeContext,
    dpi::{LogicalPosition, LogicalSize, LogicalUnit, PhysicalUnit},
    matched_style, resolve_font_props, style_align_self, style_dimension, style_flex_basis,
    style_flex_grow, style_flex_shrink, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetDC, GetTextExtentPoint32W, SelectObject},
        UI::{
            Controls::WC_BUTTON,
            WindowsAndMessaging::{
                BN_CLICKED, CreateWindowExW, DestroyWindow, SWP_NOZORDER, SendMessageW,
                SetWindowPos, SetWindowTextW, WINDOW_EX_STYLE, WM_COMMAND, WM_SETFONT, WS_CHILD,
                WS_VISIBLE,
            },
        },
    },
    core::HSTRING,
};

use crate::{AppState, WindowContext, contexts::ParentContext, font::resolved_font, utils::hiword};

const DEFAULT_PADDING_X: f32 = 10.0;
const DEFAULT_PADDING_Y: f32 = 3.0;

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__win32_Button"];

    let app_state = element.context::<AppState>().unwrap();
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );

    let title = HSTRING::from(props.title.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_BUTTON,
            &title,
            WS_VISIBLE | WS_CHILD,
            0,
            0,
            0,
            0,
            Some(parent_context.parent_hwnd),
            None,
            None,
            None,
        )
        .unwrap()
    };
    element.provide_handle(hwnd);

    let node_id = tree_context.create_node(false);
    element.on_place(closure!(
        [parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(hwnd, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(hwnd, Some(node_id));
            }
        }
    ));

    app_state.add_control_handler(
        hwnd,
        callback!([props.on_click] |msg: u32, wparam: WPARAM, _: LPARAM| {
            match msg {
                WM_COMMAND => {
                    if let Some(on_click) = on_click.get() {
                        if hiword(wparam.0 as _) as u32 == BN_CLICKED  {
                            on_click();
                        }
                    }
                },
                _ => (),
            }
        }),
    );

    element.on_unmount(closure!(
        [parent_context, app_state] || {
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
            app_state.remove_control_handler(hwnd);
        }
    ));

    scoped_effect!(
        element,
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
        element,
        [
            window_context.scale_factor,
            style_props,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color
        ] || unsafe {
            let font_props = resolve_font_props(
                style_props.get().as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(
                    resolved_font(&font_props, scale_factor.get()).0 as _,
                )),
                Some(LPARAM(1)), // redraw
            );
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.title,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let font_props = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );

            let hds = unsafe { GetDC(Some(hwnd)) };
            let title = title.get();
            let string = HSTRING::from(&title);
            unsafe {
                SetWindowTextW(hwnd, &string).unwrap();
            }

            let mesure_string = HSTRING::from(if title.is_empty() { "t" } else { &title });
            let mut size: SIZE = SIZE::default();
            unsafe {
                let font = resolved_font(&font_props, scale_factor);
                SelectObject(hds, font.into());
                GetTextExtentPoint32W(hds, &mesure_string, &mut size).unwrap();
                DeleteObject(font.into()).unwrap();
            }

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

            let width = match width {
                Dimension::Auto => LogicalUnit::new(
                    PhysicalUnit::new(size.cx).to_logical::<f32>(scale_factor).0
                        + DEFAULT_PADDING_X * 2.0,
                ),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor),
            };
            let height = match height {
                Dimension::Auto => LogicalUnit::new(
                    PhysicalUnit::new(size.cy).to_logical::<f32>(scale_factor).0
                        + DEFAULT_PADDING_Y * 2.0,
                ),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor).into(),
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
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
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
        element,
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
        element,
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
        element,
        [window_context.scale_factor, tree_context] || {
            if let Some(layout) = tree_context.layout(node_id) {
                let scale_factor = scale_factor.get();
                let point = LogicalPosition::new(layout.location.x, layout.location.y)
                    .to_physical(scale_factor);
                let size = LogicalSize::new(layout.size.width, layout.size.height)
                    .to_physical(scale_factor);

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
}
