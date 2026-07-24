use std::sync::Once;

use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{SliderProps, StyleContext, dpi::LogicalSize, matched_style};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{
        Controls::{
            ICC_BAR_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, TBM_SETPOS,
            TBM_SETRANGEMAX, TBM_SETRANGEMIN, TBS_AUTOTICKS, TRACKBAR_CLASS,
        },
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::{
            CreateWindowExW, SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_HSCROLL, WM_VSCROLL,
            WS_CHILD, WS_TABSTOP, WS_VISIBLE,
        },
    },
};

use crate::{AppState, contexts::ParentContext, native_control};

const TRACKBAR_STEPS: i32 = 10_000;
const TBM_GETPOS: u32 = 0x0400;

fn init_trackbar_class() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&controls).unwrap();
    });
}

#[component]
pub fn Slider(props: &SliderProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Slider", "__win32_Slider"];
    let app_state = element.context::<AppState>().unwrap();
    init_trackbar_class();
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
            TRACKBAR_CLASS,
            None,
            WS_VISIBLE | WS_CHILD | WS_TABSTOP | WINDOW_STYLE(TBS_AUTOTICKS),
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
    native_control::mount(
        element,
        hwnd,
        style,
        &props.view,
        create_state(LogicalSize::new(120.0, 30.0)).into_readonly(),
    );
    unsafe {
        SendMessageW(hwnd, TBM_SETRANGEMIN, Some(WPARAM(0)), Some(LPARAM(0)));
        SendMessageW(
            hwnd,
            TBM_SETRANGEMAX,
            Some(WPARAM(1)),
            Some(LPARAM(TRACKBAR_STEPS as isize)),
        );
    }

    app_state.add_control_handler(hwnd, callback!([props.minimum, props.maximum, props.on_value_change] |msg: u32, _: WPARAM, _: LPARAM| {
        if msg == WM_HSCROLL || msg == WM_VSCROLL {
            let position = unsafe { SendMessageW(hwnd, TBM_GETPOS, None, None).0 as i32 };
            if let Some(callback) = on_value_change.get() {
                callback(value_from_position(position, minimum.get(), maximum.get()));
            }
        }
    }));
    element.on_unmount(closure!(
        [app_state] || app_state.remove_control_handler(hwnd)
    ));

    scoped_effect!(
        [props.enabled]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
            }
    );
    scoped_effect!(
        [props.value, props.minimum, props.maximum]
            || unsafe {
                let position = position_from_value(value.get(), minimum.get(), maximum.get());
                SendMessageW(
                    hwnd,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(position as isize)),
                );
            }
    );
}

fn position_from_value(value: f64, minimum: f64, maximum: f64) -> i32 {
    if !value.is_finite() || !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return 0;
    }
    (((value.clamp(minimum, maximum) - minimum) / (maximum - minimum)) * TRACKBAR_STEPS as f64)
        .round() as i32
}

fn value_from_position(position: i32, minimum: f64, maximum: f64) -> f64 {
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return minimum;
    }
    minimum + (maximum - minimum) * position.clamp(0, TRACKBAR_STEPS) as f64 / TRACKBAR_STEPS as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trackbar_mapping_clamps_and_round_trips() {
        assert_eq!(position_from_value(-1.0, 0.0, 100.0), 0);
        assert_eq!(position_from_value(101.0, 0.0, 100.0), TRACKBAR_STEPS);
        assert_eq!(value_from_position(5_000, 0.0, 100.0), 50.0);
        assert_eq!(position_from_value(5.0, 10.0, 10.0), 0);
    }
}
