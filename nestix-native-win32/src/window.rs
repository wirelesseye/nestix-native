use std::sync::Once;

use nestix::{Element, Shared, component, components::ContextProvider, layout};
use nestix_native_core::WindowProps;
use windows::{
    Win32::{
        Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::{
    ParentContext,
    root::{AppContext, shared_app_state},
};

fn window_classname(hinstance: HMODULE) -> PCWSTR {
    const WINDOW_CLASSNAME: PCWSTR = w!("NestixWindowClass");
    const INIT_WINDOW_CLASS: Once = Once::new();

    INIT_WINDOW_CLASS.call_once(|| unsafe {
        let wc = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hInstance: hinstance.into(),
            lpszClassName: WINDOW_CLASSNAME,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as _),
            ..Default::default()
        };

        RegisterClassW(&wc);
    });

    WINDOW_CLASSNAME
}

#[derive(Clone)]
pub struct WindowContext {}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    let app_context = element.context::<AppContext>().unwrap();

    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };

    let title = HSTRING::from(props.title.get());

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_classname(hinstance),
            &title,
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            props.width.get() as i32,
            props.height.get() as i32,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };
    app_context
        .app_state
        .add_window(hwnd, Shared::new(WindowState::new()));

    layout! {
        ContextProvider<WindowContext>(
            .value = WindowContext {
            },
        ) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    hwnd: Some(hwnd)
                }
            ) {
                $(props.view.get())
            }
        }
    }
}

// A helper function to get the DPI for a specific monitor handle
fn get_dpi_for_monitor(h_monitor: HMONITOR) -> Option<(u32, u32)> {
    let mut dpi_x = 0;
    let mut dpi_y = 0;

    // Call the Windows API function
    let result = unsafe {
        GetDpiForMonitor(
            h_monitor,
            MDT_EFFECTIVE_DPI, // Use effective DPI to get the actual scaling applied
            &mut dpi_x,
            &mut dpi_y,
        )
    };

    if result.is_ok() {
        Some((dpi_x, dpi_y))
    } else {
        None
    }
}

// Example of how to use it within a typical application context (e.g., given a window handle)
fn get_scale_factor_for_window(hwnd: HWND) -> Option<f64> {
    // Get the monitor handle from the window handle
    let h_monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };

    if h_monitor.is_invalid() {
        return None;
    }

    if let Some((dpi_x, _dpi_y)) = get_dpi_for_monitor(h_monitor) {
        // The standard base DPI is 96 (100% scale)
        let scale_factor = dpi_x as f64 / 96.0;
        Some(scale_factor)
    } else {
        None
    }
}

pub(crate) struct WindowState {
    bg_brush: HBRUSH,
}

impl WindowState {
    fn new() -> Self {
        Self {
            bg_brush: unsafe { GetSysColorBrush(COLOR_BTNFACE) },
        }
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CTLCOLORSTATIC => {
                let app_state = shared_app_state();
                let window_state = app_state.window_state(hwnd).unwrap();

                let hdc = HDC(wparam.0 as _);
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(GetSysColor(COLOR_BTNTEXT)));
                LRESULT(window_state.bg_brush.0 as isize)
            }

            // WM_COMMAND => {
            //     let control_id = loword(wparam.0 as u32);
            //     if control_id == loword(BUTTON_HMENU.unwrap().0 as u32) {
            //         SetWindowTextW(LABEL_HWND.unwrap(), w!("Button clicked!"));
            //     }
            //     LRESULT(0)
            // }
            WM_DESTROY => {
                let app_state = shared_app_state();
                app_state.remove_window(hwnd);

                if app_state.quit_when_all_windows_closed() && !app_state.has_windows() {
                    app_state.quit();
                }

                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
