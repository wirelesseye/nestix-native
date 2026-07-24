use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use nestix::{Element, PropValue, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    RadioButtonProps, StyleContext,
    dpi::{LogicalSize, PhysicalUnit},
    matched_style, resolve_font_props,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{
            DeleteObject, GetDC, GetTextExtentPoint32W, HFONT, ReleaseDC, SelectObject,
        },
        UI::{
            Controls::{BCM_GETIDEALSIZE, BST_CHECKED, BST_UNCHECKED, WC_BUTTON},
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                BM_SETCHECK, BN_CLICKED, BS_RADIOBUTTON, CreateWindowExW, SendMessageW,
                SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_SETFONT, WS_CHILD,
                WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
    core::HSTRING,
};

use crate::{
    AppState, WindowContext, contexts::ParentContext, font::resolved_font, native_control,
    utils::hiword,
};

struct RegisteredRadio {
    window: HWND,
    hwnd: HWND,
    group: PropValue<String>,
}

thread_local! {
    static RADIOS: RefCell<Vec<RegisteredRadio>> = const { RefCell::new(Vec::new()) };
}

#[component]
/// Renders a native Win32 radio button.
pub fn RadioButton(props: &RadioButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__RadioButton", "__win32_RadioButton"];
    let app_state = element.context::<AppState>().unwrap();
    let window = element.context::<WindowContext>().unwrap();
    let window_hwnd = window.hwnd;
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
            WS_VISIBLE | WS_CHILD | WS_TABSTOP | WINDOW_STYLE(BS_RADIOBUTTON as u32),
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

    RADIOS.with_borrow_mut(|radios| {
        radios.push(RegisteredRadio {
            window: window_hwnd,
            hwnd,
            group: props.group.clone(),
        })
    });
    app_state.add_control_handler(hwnd, callback!([props.group, props.on_select] |msg: u32, wparam: WPARAM, _: LPARAM| {
        if msg == WM_COMMAND && hiword(wparam.0 as _) as u32 == BN_CLICKED {
            let selected_group = group.get();
            RADIOS.with_borrow(|radios| {
                for radio in radios {
                    if radio.window == window_hwnd && radio.group.get() == selected_group {
                        unsafe { SendMessageW(radio.hwnd, BM_SETCHECK, Some(WPARAM(BST_UNCHECKED.0 as usize)), None); }
                    }
                }
            });
            unsafe { SendMessageW(hwnd, BM_SETCHECK, Some(WPARAM(BST_CHECKED.0 as usize)), None); }
            if let Some(callback) = on_select.get() { callback(); }
        }
    }));

    let font = Rc::new(Cell::new(None::<HFONT>));
    element.on_unmount(closure!(
        [app_state, font] || {
            RADIOS.with_borrow_mut(|radios| radios.retain(|radio| radio.hwnd != hwnd));
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
        [props.enabled, props.selected]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
                SendMessageW(
                    hwnd,
                    BM_SETCHECK,
                    Some(WPARAM(if selected.get() {
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
            let mut text_size = SIZE::default();
            let measure = HSTRING::from(if title.is_empty() { "t" } else { &title });
            GetTextExtentPoint32W(dc, &measure, &mut text_size).unwrap();
            SelectObject(dc, old);
            ReleaseDC(Some(hwnd), dc);

            // Include the themed radio glyph, its spacing, and the trailing
            // inset. A fixed allowance clips labels under some DPI/themes.
            let mut ideal_size = SIZE::default();
            let has_ideal_size = SendMessageW(
                hwnd,
                BCM_GETIDEALSIZE,
                None,
                Some(LPARAM((&raw mut ideal_size) as isize)),
            )
            .0 != 0;
            let measured_size = if has_ideal_size {
                ideal_size
            } else {
                SIZE {
                    cx: text_size.cx + 28,
                    cy: (text_size.cy + 6).max(20),
                }
            };
            intrinsic.set(LogicalSize::new(
                PhysicalUnit::new(measured_size.cx)
                    .to_logical::<f32>(scale)
                    .0,
                PhysicalUnit::new(measured_size.cy)
                    .to_logical::<f32>(scale)
                    .0,
            ));
        }
    );
}
