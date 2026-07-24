use std::{cell::Cell, rc::Rc, sync::Once};

use nestix::{
    Element, Layout, PropValue, Readonly, Shared, callback, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    StyleScope, TitleBarMode, TreeContext, WindowProps,
    dpi::{LogicalSize, PhysicalSize, Size},
};
use taffy::{NodeId, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::{DRAWITEMSTRUCT, NMHDR},
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::{AppState, contexts::ParentContext, font::colorref, root::shared_app_state};

fn window_classname(hinstance: HMODULE) -> PCWSTR {
    const WINDOW_CLASSNAME: PCWSTR = w!("NestixNativeWindow");
    static INIT_WINDOW_CLASS: Once = Once::new();

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
/// Context provided to descendants of a Win32 window.
pub struct WindowContext {
    /// The window's current display scale relative to 96 DPI.
    pub scale_factor: Readonly<f64>,
    pub(crate) hwnd: HWND,
}

#[component]
/// Creates a native Win32 top-level window and renders its contents.
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__win32_Window"];

    let app_state = element.context::<AppState>().unwrap();

    let scale_factor = create_state(1.0);

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
    let window_context = Rc::new(WindowContext {
        scale_factor: scale_factor.clone().into_readonly(),
        hwnd,
    });

    let window_state = Rc::new(WindowState {
        bg_brush: unsafe { GetSysColorBrush(COLOR_BTNFACE) },
        tree_context: tree_context.clone(),
        root_view: Cell::new(None),
        on_resize: props.on_resize.clone(),
        on_close_requested: props.on_close_requested.clone(),
    });
    app_state.add_window(hwnd, window_state.clone());

    element.on_unmount(move || unsafe {
        DestroyWindow(hwnd).unwrap();
    });

    if let Some(value) = get_scale_factor_for_window(hwnd) {
        scale_factor.set(value);
    }

    scoped_effect!(
        [props.title]
            || unsafe {
                SetWindowTextW(hwnd, &HSTRING::from(title.get())).unwrap();
            }
    );

    scoped_effect!(
        [props.title_bar_mode]
            || unsafe {
                apply_title_bar_mode(hwnd, title_bar_mode.get());
            }
    );

    scoped_effect!(
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
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                    ContextProvider<ParentContext>(
                        ParentContext {
                            parent_hwnd: hwnd,
                            add_child: Some(callback!([] |child_hwnd: HWND,
                            child_node: Option<NodeId> | {
                                tree_context.set_root_node(child_node);
                                window_state.root_view.set(Some(child_hwnd));
                                let mut client_rect: RECT = RECT::default();
                                unsafe {
                                    GetClientRect(hwnd, &mut client_rect).unwrap();
                                }
                                let width = client_rect.right - client_rect.left;
                                let height = client_rect.bottom - client_rect.top;
                                unsafe {
                                    SetWindowPos(
                                        child_hwnd,
                                        None,
                                        0,
                                        0,
                                        width,
                                        height,
                                        SWP_NOZORDER,
                                    )
                                    .unwrap();
                                }
                                let size: LogicalSize<f32> =
                                    PhysicalSize::new(width, height).to_logical(scale_factor.get());
                                if let Some(child_node) = child_node {
                                    tree_context.update_style(child_node, |prev| Style {
                                        size: taffy::Size {
                                            width: taffy::Dimension::from_length(size.width),
                                            height: taffy::Dimension::from_length(size.height),
                                        },
                                        ..prev
                                    });
                                    tree_context.refresh();
                                }
                            })),
                            insert_child: None,
                            remove_child: None,
                            parent_node: None
                        },
                    ) {
                        $(props.children.clone().map(|element| Layout::from(element.clone())))
                    }
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
    on_close_requested: PropValue<Option<Shared<dyn Fn()>>>,
}

unsafe fn apply_title_bar_mode(hwnd: HWND, mode: TitleBarMode) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next_style = match mode {
            TitleBarMode::Hidden => style & !(WS_CAPTION.0 as isize),
            // Custom title-bar overlays are not supported by this backend.
            TitleBarMode::System | TitleBarMode::Overlay => style | WS_CAPTION.0 as isize,
        };
        if next_style != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next_style);
        }

        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        )
        .unwrap();
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if crate::menu::handle_menu_shortcut(hwnd, wparam.0) {
                    return LRESULT(0);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_CLOSE => {
                let app_state = shared_app_state();
                if let Some(window_state) = app_state.window_state(hwnd)
                    && let Some(on_close_requested) = window_state.on_close_requested.get()
                {
                    on_close_requested();
                }
                LRESULT(0)
            }

            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                let app_state = shared_app_state();
                let window_state = app_state.window_state(hwnd).unwrap();

                let hdc = HDC(wparam.0 as _);
                SetBkMode(hdc, TRANSPARENT);
                let control = HWND(lparam.0 as _);
                let color = app_state
                    .control_text_color(control)
                    .map(colorref)
                    .unwrap_or_else(|| COLORREF(GetSysColor(COLOR_BTNTEXT)));
                SetTextColor(hdc, color);

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
                let control = HWND(lparam.0 as _);
                if control.0.is_null() {
                    crate::menu::handle_menu_command(hwnd, wparam.0 & 0xffff);
                } else {
                    app_state.handle_control_event(control, msg, wparam, lparam);
                }

                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_HSCROLL | WM_VSCROLL => {
                let app_state = shared_app_state();
                let control = HWND(lparam.0 as _);
                if !control.0.is_null() {
                    app_state.handle_control_event(control, msg, wparam, lparam);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_DRAWITEM => {
                let app_state = shared_app_state();
                let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
                app_state.handle_control_event(item.hwndItem, msg, wparam, lparam);
                LRESULT(1)
            }

            WM_DESTROY => {
                let app_state = shared_app_state();
                app_state.remove_window(hwnd);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
