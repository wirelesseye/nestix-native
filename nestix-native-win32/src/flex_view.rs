use std::sync::Once;

use nestix::{Element, callback, closure, component, components::ContextProvider, effect, layout};
use nestix_native_core::{
    Alignment, Direction, ExtendsViewProps, FlexViewProps, TreeContext, Wrap,
    dpi::{LogicalPosition, LogicalSize},
};
use taffy::{NodeId, Size, Style};
use windows::{
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{COLOR_BTNFACE, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::NMHDR,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW,
                LoadCursorW, RegisterClassW, SWP_NOZORDER, SetWindowPos, WINDOW_EX_STYLE,
                WM_COMMAND, WM_NOTIFY, WNDCLASSW, WS_CHILD, WS_VISIBLE,
            },
        },
    },
    core::{PCWSTR, w},
};

use crate::{WindowContext, contexts::ParentContext, shared_app_state};

fn window_classname(hinstance: HMODULE) -> PCWSTR {
    const WINDOW_CLASSNAME: PCWSTR = w!("NestixNativeFlexView");
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

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_classname(hinstance),
            None,
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
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
        }
    ));

    effect!(
        [tree_context, props.grow()] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: grow.get(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [
            window_context,
            tree_context,
            parent_context.parent_node,
            props.width(),
            props.height(),
        ] || {
            let scale_factor = window_context.scale_factor.get();

            if parent_node.is_some() {
                // Update size when the node is not root
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.get().into_taffy_dimension(scale_factor),
                        height: height.get().into_taffy_dimension(scale_factor),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    effect!(
        [tree_context, props.direction] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: match direction.get() {
                    Direction::Row => taffy::FlexDirection::Row,
                    Direction::RowReverse => taffy::FlexDirection::RowReverse,
                    Direction::Column => taffy::FlexDirection::Column,
                    Direction::ColumnReverse => taffy::FlexDirection::ColumnReverse,
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [tree_context, props.alignment] || {
            tree_context.update_style(node_id, |prev| Style {
                align_items: match alignment.get() {
                    Alignment::Unset => None,
                    Alignment::FlexStart => Some(taffy::AlignItems::FlexStart),
                    Alignment::FlexEnd => Some(taffy::AlignItems::FlexEnd),
                    Alignment::Center => Some(taffy::AlignItems::Center),
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [tree_context, props.wrap] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: match wrap.get() {
                    Wrap::NoWrap => taffy::FlexWrap::NoWrap,
                    Wrap::Wrap => taffy::FlexWrap::Wrap,
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node
        ] || {
            if parent_node.is_some() {
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
        }
    );

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                parent_hwnd: hwnd,
                add_child: Some(callback!([tree_context] |_: HWND, child_node: Option<NodeId>| {
                    if let Some(child_node) = child_node {
                        tree_context.add_child(node_id, child_node);
                    }
                })),
                remove_child: Some(callback!([tree_context] |_: HWND, child_node: Option<NodeId>| {
                    if let Some(child_node) = child_node {
                        tree_context.remove_child(node_id, child_node);
                    }
                })),
                parent_node: Some(node_id),
            },
        ) {
            $(props.children.clone())
        }
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
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

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
