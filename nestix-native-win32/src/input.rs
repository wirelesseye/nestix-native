use std::{cell::Cell, rc::Rc};

use nestix::{Element, callback, closure, component, effect};
use nestix_native_core::{
    Dimension, InputProps, TreeContext, ViewPropsExt,
    dpi::{LogicalPosition, LogicalSize, PhysicalUnit},
};
use taffy::{Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetDC, GetTextExtentPoint32W, SelectObject},
        UI::{
            Controls::WC_EDIT,
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, EN_CHANGE, ES_AUTOHSCROLL, GetWindowTextLengthW,
                GetWindowTextW, SWP_NOZORDER, SendMessageW, SetWindowPos, SetWindowTextW,
                WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_SETFONT, WS_BORDER, WS_CHILD,
                WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
    core::HSTRING,
};

use crate::{AppState, WindowContext, contexts::ParentContext, font::ui_font, utils::hiword};

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    let app_state = element.context::<AppState>().unwrap();
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let is_setting_value = Rc::new(Cell::new(false));
    let is_mounted = Rc::new(Cell::new(true));

    let value = HSTRING::from(props.value.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_EDIT,
            &value,
            WS_VISIBLE | WS_CHILD | WS_TABSTOP | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
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

    let node_id = tree_context.create_node(true);
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
        callback!([props.on_text_change, is_setting_value, is_mounted] |msg: u32, wparam: WPARAM, _: LPARAM| {
            if msg == WM_COMMAND
                && hiword(wparam.0 as _) as u32 == EN_CHANGE
                && is_mounted.get()
                && !is_setting_value.get()
                && let Some(on_text_change) = on_text_change.get()
            {
                let text = window_text(hwnd);
                on_text_change(&text);
            }
        }),
    );

    element.on_unmount(closure!(
        [parent_context, is_mounted] || {
            is_mounted.set(false);
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
            app_state.remove_control_handler(hwnd);
        }
    ));

    effect!(
        [window_context.scale_factor, is_mounted]
            || unsafe {
                if !is_mounted.get() {
                    return;
                }

                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(ui_font(12.0, scale_factor.get()).0 as _)),
                    Some(LPARAM(1)), // redraw
                );
            }
    );

    effect!(
        [props.value, is_setting_value, is_mounted] || {
            if !is_mounted.get() {
                return;
            }

            let next_value = value.get();
            if window_text(hwnd) != next_value {
                is_setting_value.set(true);
                unsafe {
                    SetWindowTextW(hwnd, &HSTRING::from(next_value)).unwrap();
                }
                is_setting_value.set(false);
            }
        }
    );

    effect!(
        [tree_context, props.grow(), is_mounted] || {
            if !is_mounted.get() {
                return;
            }

            tree_context.update_style(node_id, |prev| Style {
                flex_grow: grow.get(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [
            window_context.scale_factor,
            tree_context,
            props.value,
            props.width(),
            props.height(),
            is_mounted
        ] || {
            if !is_mounted.get() {
                return;
            }

            let scale_factor = scale_factor.get();

            let hds = unsafe { GetDC(Some(hwnd)) };
            let text = HSTRING::from(value.get());
            let mut size: SIZE = SIZE::default();
            unsafe {
                let font = ui_font(12.0, scale_factor);
                SelectObject(hds, font.into());
                GetTextExtentPoint32W(hds, &text, &mut size).unwrap();
                DeleteObject(font.into()).unwrap();
            }

            let width = match width.get() {
                Dimension::Auto => PhysicalUnit::new(size.cx + 12).to_logical(scale_factor),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor),
            };
            let height = match height.get() {
                Dimension::Auto => PhysicalUnit::new(size.cy + 8).to_logical(scale_factor),
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

    effect!(
        [window_context.scale_factor, tree_context, is_mounted] || {
            if !is_mounted.get() {
                return;
            }

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

fn window_text(hwnd: windows::Win32::Foundation::HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0; len as usize + 1];
    let read = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..read as usize])
}
