use std::{cell::Cell, rc::Rc};

use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    CheckboxProps, StyleContext,
    dpi::{LogicalSize, PhysicalUnit},
    matched_style, resolve_font_props,
};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{
            DeleteObject, GetDC, GetTextExtentPoint32W, HFONT, ReleaseDC, SelectObject,
        },
        UI::{
            Controls::{BST_CHECKED, BST_UNCHECKED, WC_BUTTON},
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, CreateWindowExW,
                SendMessageW, SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND,
                WM_SETFONT, WS_CHILD, WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
    core::HSTRING,
};

use crate::{
    AppState, WindowContext, contexts::ParentContext, font::resolved_font, native_control,
    utils::hiword,
};

#[component]
/// Renders a native Win32 checkbox.
pub fn Checkbox(props: &CheckboxProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Checkbox", "__win32_Checkbox"];
    let app_state = element.context::<AppState>().unwrap();
    let window = element.context::<WindowContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_BUTTON,
            &HSTRING::from(props.title.get()),
            WS_VISIBLE | WS_CHILD | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            0,
            0,
            0,
            0,
            Some(parent.parent_hwnd),
            None,
            None,
            None,
        )
        .unwrap()
    };
    let intrinsic = create_state(LogicalSize::new(24.0, 22.0));
    native_control::mount(
        element,
        hwnd,
        style.clone(),
        &props.view,
        intrinsic.clone().into_readonly(),
    );

    app_state.add_control_handler(hwnd, callback!([props.checked, props.on_checked_change] |msg: u32, wparam: WPARAM, _: LPARAM| {
        if msg == WM_COMMAND && hiword(wparam.0 as _) as u32 == BN_CLICKED {
            let clicked_checked = unsafe { SendMessageW(hwnd, BM_GETCHECK, None, None).0 as u32 == BST_CHECKED.0 };
            if let Some(callback) = on_checked_change.get() { callback(clicked_checked); }
            unsafe {
                SendMessageW(hwnd, BM_SETCHECK, Some(WPARAM(if checked.get() { BST_CHECKED.0 as usize } else { BST_UNCHECKED.0 as usize })), None);
            }
        }
    }));

    let font = Rc::new(Cell::new(None::<HFONT>));
    element.on_unmount(closure!(
        [app_state, font] || {
            app_state.remove_control_handler(hwnd);
            app_state.set_control_text_color(hwnd, None);
            if let Some(font) = font.take() {
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    ));

    scoped_effect!(
        [props.enabled, props.checked]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
                SendMessageW(
                    hwnd,
                    BM_SETCHECK,
                    Some(WPARAM(if checked.get() {
                        BST_CHECKED.0 as usize
                    } else {
                        BST_UNCHECKED.0 as usize
                    })),
                    None,
                );
            }
    );

    scoped_effect!(
        [
            window.scale_factor,
            style,
            props.title,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            intrinsic,
            font
        ] || unsafe {
            let scale = scale_factor.get();
            let resolved = resolve_font_props(
                style.get().as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let next_font = resolved_font(&resolved, scale);
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(next_font.0 as _)),
                Some(LPARAM(1)),
            );
            if let Some(previous) = font.replace(Some(next_font)) {
                let _ = DeleteObject(previous.into());
            }
            app_state.set_control_text_color(hwnd, resolved.text_color);
            let title = title.get();
            SetWindowTextW(hwnd, &HSTRING::from(&title)).unwrap();
            let dc = GetDC(Some(hwnd));
            let old = SelectObject(dc, next_font.into());
            let mut size = SIZE::default();
            let measure = HSTRING::from(if title.is_empty() { "t" } else { &title });
            GetTextExtentPoint32W(dc, &measure, &mut size).unwrap();
            SelectObject(dc, old);
            ReleaseDC(Some(hwnd), dc);
            intrinsic.set(LogicalSize::new(
                PhysicalUnit::new(size.cx + 24).to_logical::<f32>(scale).0,
                PhysicalUnit::new((size.cy + 6).max(20))
                    .to_logical::<f32>(scale)
                    .0,
            ));
        }
    );
}
