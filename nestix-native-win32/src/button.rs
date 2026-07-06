use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    ButtonProps, Dimension, StyleContext, TreeContext,
    dpi::{LogicalPosition, LogicalSize, LogicalUnit, PhysicalUnit},
    matched_style, style_align_self, style_dimension, style_margin,
    utils::margin_to_taffy,
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

use crate::{AppState, WindowContext, contexts::ParentContext, font::ui_font, utils::hiword};

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
        [parent_context] || {
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
        [window_context.scale_factor]
            || unsafe {
                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(ui_font(12.0, scale_factor.get()).0 as _)),
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
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            let hds = unsafe { GetDC(Some(hwnd)) };
            let title = title.get();
            let string = HSTRING::from(&title);
            unsafe {
                SetWindowTextW(hwnd, &string).unwrap();
            }

            let mesure_string = HSTRING::from(if title.is_empty() { "t" } else { &title });
            let mut size: SIZE = SIZE::default();
            unsafe {
                let font = ui_font(12.0, scale_factor);
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
