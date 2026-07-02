use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Once};

use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension, FlexViewProps, StyleContext, StyleScope, TreeContext,
    dpi::{LogicalPosition, LogicalSize},
    matched_style, style_align_items, style_align_self, style_dimension, style_flex_direction,
    style_flex_wrap, style_grow, style_margin,
};
use taffy::{NodeId, Size, Style};
use windows::{
    Win32::{
        Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            COLOR_BTNFACE, CreateSolidBrush, DeleteObject, FillRect, HBRUSH, InvalidateRect,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::NMHDR,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
                GetClientRect, IDC_ARROW, LoadCursorW, RegisterClassW, SWP_NOZORDER, SetWindowPos,
                WINDOW_EX_STYLE, WM_COMMAND, WM_ERASEBKGND, WM_NOTIFY, WNDCLASSW, WS_CHILD,
                WS_VISIBLE,
            },
        },
    },
    core::{PCWSTR, w},
};

use crate::{WindowContext, contexts::ParentContext, shared_app_state, utils::margin_to_taffy};

thread_local! {
    static BACKGROUND_BRUSHES: RefCell<HashMap<*mut std::ffi::c_void, HBRUSH>> =
        RefCell::new(HashMap::new());
}

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
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        props.class.clone(),
        &["__FlexView", "__win32_FlexView"],
    );
    let child_nodes = Rc::new(RefCell::new(Vec::new()));

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
    element.provide_handle(hwnd);

    let node_id = tree_context.create_node(false);
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

    element.on_unmount(closure!(
        [parent_context] || {
            if let Some(brush) =
                BACKGROUND_BRUSHES.with_borrow_mut(|brushes| brushes.remove(&hwnd.0))
            {
                unsafe {
                    DeleteObject(brush.into()).unwrap();
                }
            }
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [style_props, props.bg_color] || {
            let style_props = style_props.get();
            if let Some(brush) =
                BACKGROUND_BRUSHES.with_borrow_mut(|brushes| brushes.remove(&hwnd.0))
            {
                unsafe {
                    DeleteObject(brush.into()).unwrap();
                }
            }

            let bg_color = bg_color.get().or_else(|| {
                style_props
                    .as_ref()
                    .and_then(|style_props| style_props.bg_color)
            });
            if let Some(bg_color) = bg_color {
                let rgb = bg_color.into_rgb();
                if rgb.alpha > 0 {
                    let color = COLORREF(
                        rgb.red as u32 | ((rgb.green as u32) << 8) | ((rgb.blue as u32) << 16),
                    );
                    let brush = unsafe { CreateSolidBrush(color) };
                    BACKGROUND_BRUSHES.with_borrow_mut(|brushes| {
                        brushes.insert(hwnd.0, brush);
                    });
                }
            }

            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, true);
            }
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.grow] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_grow(style_props.as_ref(), grow.get()),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context,
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = window_context.scale_factor.get();
            let style_props = style_props.get();
            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            );

            if parent_node.is_some() {
                // Update size when the node is not root
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.flex_direction] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: style_flex_direction(style_props.as_ref(), flex_direction.get())
                    .to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.align_items] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_items: style_align_items(style_props.as_ref(), align_items.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.flex_wrap] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: style_flex_wrap(style_props.as_ref(), flex_wrap.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
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
        StyleScope(.class = props.class.clone(), .default_classes = ["__FlexView", "__win32_FlexView"]) {
            ContextProvider<ParentContext>(
                ParentContext {
                    parent_hwnd: hwnd,
                    add_child: Some(callback!([tree_context, child_nodes] |_: HWND, child_node: Option<NodeId>| {
                        if let Some(child_node) = child_node {
                            if child_nodes.borrow().contains(&child_node) {
                                tree_context.remove_child(node_id, child_node);
                                child_nodes.borrow_mut().retain(|node| *node != child_node);
                            }
                            tree_context.add_child(node_id, child_node);
                            child_nodes.borrow_mut().push(child_node);
                            tree_context.refresh();
                        }
                    })),
                    insert_child: Some(callback!([tree_context, child_nodes] |_: HWND, child_node: Option<NodeId>, index: usize| {
                        if let Some(child_node) = child_node {
                            if child_nodes.borrow().contains(&child_node) {
                                tree_context.remove_child(node_id, child_node);
                                child_nodes.borrow_mut().retain(|node| *node != child_node);
                            }
                            let index = index.min(child_nodes.borrow().len());
                            tree_context.insert_child(node_id, child_node, index);
                            child_nodes.borrow_mut().insert(index, child_node);
                            tree_context.refresh();
                        }
                    })),
                    remove_child: Some(callback!([tree_context, child_nodes] |_: HWND, child_node: Option<NodeId>| {
                        if let Some(child_node) = child_node {
                            child_nodes.borrow_mut().retain(|node| *node != child_node);
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
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_ERASEBKGND => {
                if let Some(brush) =
                    BACKGROUND_BRUSHES.with_borrow(|brushes| brushes.get(&hwnd.0).copied())
                {
                    let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as _);
                    let mut rect = RECT::default();
                    GetClientRect(hwnd, &mut rect).unwrap();
                    FillRect(hdc, &rect, brush);

                    LRESULT(1)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
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

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
