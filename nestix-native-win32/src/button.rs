use nestix::{Element, component, effect};
use nestix_native_core::{
    ButtonProps, Dimension, ExtendsViewProps, TreeContext,
    dpi::{LogicalPosition, LogicalSize, PhysicalUnit},
};
use taffy::{Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetDC, GetTextExtentPoint32W, SelectObject},
        UI::WindowsAndMessaging::{
            CreateWindowExW, SWP_NOZORDER, SendMessageW, SetWindowPos, WINDOW_EX_STYLE, WM_SETFONT,
            WS_CHILD, WS_VISIBLE,
        },
    },
    core::{HSTRING, w},
};

use crate::{WindowContext, contexts::ParentContext, font::ui_font};

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let title = HSTRING::from(props.title.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
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

    let node_id = tree_context.create_node(false);
    if let Some(add_child) = &parent_context.add_child {
        add_child(hwnd, Some(node_id));
    }

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
            props.title,
            props.width(),
            props.height()
        ] || {
            let scale_factor = scale_factor.get();

            let hds = unsafe { GetDC(Some(hwnd)) };
            let text = HSTRING::from(title.get());
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
            tree_context.update();
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
