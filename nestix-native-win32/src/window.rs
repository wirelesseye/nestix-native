use std::{cell::Cell, rc::Rc, sync::Once};

use nestix::{
    Element, PropValue, Readonly, Shared, callback, component, components::ContextProvider,
    create_state, effect, layout,
};
use nestix_native_core::{
    TreeContext, WindowProps,
    dpi::{LogicalSize, PhysicalSize, Size},
};
use taffy::{NodeId, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::NMHDR,
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::{AppState, contexts::ParentContext, root::shared_app_state};

fn window_classname(hinstance: HMODULE) -> PCWSTR {
    const WINDOW_CLASSNAME: PCWSTR = w!("NestixNativeWindow");
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
pub struct WindowContext {
    pub scale_factor: Readonly<f64>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    let app_state = element.context::<AppState>().unwrap();

    let scale_factor = create_state(1.0);

    let window_context = Rc::new(WindowContext {
        scale_factor: scale_factor.clone().into_readonly(),
    });
    let tree_context = Rc::new(TreeContext::new());

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
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };

    let window_state = Rc::new(WindowState {
        bg_brush: unsafe { GetSysColorBrush(COLOR_BTNFACE) },
        tree_context: tree_context.clone(),
        root_view: Cell::new(None),
        on_resize: props.on_resize.clone(),
    });
    app_state.add_window(hwnd, window_state.clone());

    if let Some(value) = get_scale_factor_for_window(hwnd) {
        scale_factor.set(value);
    }

    effect!(
        [scale_factor, props.width, props.height]
            || unsafe {
                let mut rect_client = RECT::default();
                let mut rect_wind = RECT::default();
                let mut point_diff = POINT::default();

                GetClientRect(hwnd, &mut rect_client).unwrap();
                GetWindowRect(hwnd, &mut rect_wind).unwrap();

                point_diff.x = (rect_wind.right - rect_wind.left) - rect_client.right;
                point_diff.y = (rect_wind.bottom - rect_wind.top) - rect_client.bottom;

                let size: PhysicalSize<i32> =
                    LogicalSize::new(width.get(), height.get()).to_physical(scale_factor.get());

                MoveWindow(
                    hwnd,
                    rect_wind.left,
                    rect_wind.top,
                    size.width + point_diff.x,
                    size.height + point_diff.y,
                    true,
                )
                .unwrap();
            }
    );

    layout! {
        ContextProvider<WindowContext>(
            .value = window_context,
        ) {
            ContextProvider<TreeContext>(
                .value = tree_context.clone(),
            ) {
                ContextProvider<ParentContext>(
                    .value = ParentContext {
                        parent_hwnd: hwnd,
                        add_child: Some(callback!([] |child_hwnd: HWND, child_node: Option<NodeId>| {
                            tree_context.set_root_node(child_node);
                            window_state.root_view.set(Some(child_hwnd));

                            let mut client_rect: RECT = RECT::default();
                            unsafe { GetClientRect(hwnd, &mut client_rect).unwrap(); }

                            let width = client_rect.right - client_rect.left;
                            let height = client_rect.bottom - client_rect.top;

                            unsafe {
                                SetWindowPos(child_hwnd, None, 0, 0, width, height, SWP_NOZORDER)
                                    .unwrap();
                            }

                            let size: LogicalSize<f32> = PhysicalSize::new(width, height).to_logical(scale_factor.get());
                            if let Some(child_node) = child_node {
                                tree_context.update_style(child_node, |prev| Style {
                                    size: taffy::Size {
                                        width: taffy::Dimension::from_length(size.width),
                                        height: taffy::Dimension::from_length(size.height)
                                    },
                                    ..prev
                                });
                            }
                        })),
                        remove_child: None,
                        parent_node: None,
                    }
                ) {
                    $(props.children.get())
                }
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
    tree_context: Rc<TreeContext>,
    root_view: Cell<Option<HWND>>,
    on_resize: PropValue<Option<Shared<dyn Fn(Size)>>>,
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

            WM_SIZE => {
                let app_state = shared_app_state();
                if let Some(window_state) = app_state.window_state(hwnd) {
                    let mut client_rect: RECT = RECT::default();
                    GetClientRect(hwnd, &mut client_rect).unwrap();

                    let width = client_rect.right - client_rect.left;
                    let height = client_rect.bottom - client_rect.top;

                    if let Some(root_view) = window_state.root_view.get() {
                        SetWindowPos(root_view, None, 0, 0, width, height, SWP_NOZORDER).unwrap();
                    }

                    if let Some(root_node) = window_state.tree_context.root_node() {
                        let scale_factor = get_scale_factor_for_window(hwnd).unwrap();
                        let size: LogicalSize<f32> =
                            PhysicalSize::new(width, height).to_logical(scale_factor);
                        window_state
                            .tree_context
                            .update_style(root_node, |prev| Style {
                                size: taffy::Size {
                                    width: taffy::Dimension::from_length(size.width),
                                    height: taffy::Dimension::from_length(size.height),
                                },
                                ..prev
                            });
                        window_state.tree_context.refresh();
                    }

                    if let Some(on_resize) = window_state.on_resize.get() {
                        on_resize(Size::Physical(PhysicalSize::new(
                            width as u32,
                            height as u32,
                        )))
                    }
                }

                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_NOTIFY => {
                let app_state = shared_app_state();
                let phdr = &*(lparam.0 as *const NMHDR);
                app_state.handle_control_event(phdr.hwndFrom, msg, wparam, lparam);

                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_COMMAND => {
                let app_state = shared_app_state();
                let hwnd = HWND(lparam.0 as _);
                app_state.handle_control_event(hwnd, msg, wparam, lparam);

                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

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
