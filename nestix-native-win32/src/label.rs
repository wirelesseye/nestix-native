use nestix::{Element, closure, component, effect};
use nestix_native_core::{
    Dimension, ExtendsViewProps, LabelProps, TreeContext,
    dpi::{LogicalPosition, LogicalSize, PhysicalUnit},
};
use taffy::{Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetDC, GetTextExtentPoint32W, SelectObject},
        UI::{
            Controls::WC_STATIC,
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, SWP_NOZORDER, SendMessageW, SetWindowPos, SetWindowTextW, WINDOW_EX_STYLE, WM_SETFONT, WS_CHILD, WS_VISIBLE
            },
        },
    },
    core::HSTRING,
};

use crate::{WindowContext, contexts::ParentContext, font::ui_font};

#[component]
pub fn Label(props: &LabelProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let text = HSTRING::from(props.text.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_STATIC,
            &text,
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

    let node_id = tree_context.create_node(false);
    if let Some(add_child) = &parent_context.add_child {
        add_child(hwnd, Some(node_id));
    }

    element.on_destroy(closure!(
        [parent_context] || {
            unsafe { DestroyWindow(hwnd).unwrap(); }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
        }
    ));

    effect!(
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

    effect!(
        [
            window_context.scale_factor,
            tree_context,
            props.text,
            props.width(),
            props.height()
        ] || {
            let scale_factor = scale_factor.get();

            let hds = unsafe { GetDC(Some(hwnd)) };
            let text = HSTRING::from(text.get());
            unsafe {
                SetWindowTextW(hwnd, &text).unwrap();
            }

            let mut size: SIZE = SIZE::default();
            unsafe {
                let font = ui_font(12.0, scale_factor);
                SelectObject(hds, font.into());
                GetTextExtentPoint32W(hds, &text, &mut size).unwrap();
                DeleteObject(font.into()).unwrap();
            }

            let width = match width.get() {
                Dimension::Auto => PhysicalUnit::new(size.cx).to_logical(scale_factor),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor),
            };
            let height = match height.get() {
                Dimension::Auto => PhysicalUnit::new(size.cy).to_logical(scale_factor),
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
