use std::{cell::RefCell, rc::Rc, sync::Once};

use nestix::{
    Element, State, callback, closure, component, components::ContextProvider, create_state,
    effect, layout,
};
use nestix_native_core::{
    ExtendsViewProps, TabViewItemProps, TabViewProps, TreeContext,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        UI::{
            Controls::{
                ICC_TAB_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, NMHDR, TCIF_TEXT,
                TCITEMW, TCM_ADJUSTRECT, TCM_GETCURSEL, TCM_GETITEMCOUNT, TCM_INSERTITEM,
                TCN_SELCHANGE, WC_TABCONTROL,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, GetClientRect, SW_HIDE, SW_SHOW, SWP_NOZORDER,
                SendMessageW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WM_NOTIFY, WM_SETFONT,
                WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
            },
        },
    },
    core::PWSTR,
};

use crate::{AppState, WindowContext, contexts::ParentContext, font::ui_font};

fn init_common_controls() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let mut icex = INITCOMMONCONTROLSEX::default();
        icex.dwICC = ICC_TAB_CLASSES;
        unsafe {
            let _ = InitCommonControlsEx(&icex);
        };
    });
}

struct TabViewContext {
    current_selected: State<Option<String>>,
    tab_ids: RefCell<Vec<String>>,
}

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    let app_state = element.context::<AppState>().unwrap();
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let current_selected = create_state(None);

    init_common_controls();
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_TABCONTROL,
            None,
            WS_CHILD | WS_CLIPSIBLINGS | WS_VISIBLE,
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

    let node_id = tree_context.create_node(true);
    if let Some(add_child) = &parent_context.add_child {
        add_child(hwnd, Some(node_id));
    }

    let tab_view_context = Rc::new(TabViewContext {
        current_selected: current_selected.clone(),
        tab_ids: RefCell::new(Vec::new()),
    });

    app_state.add_control_handler(
        hwnd,
        callback!([tab_view_context] |msg: u32, _: WPARAM, lparam: LPARAM| {
            match msg {
                WM_NOTIFY => unsafe {
                    let nmhdr = &*(lparam.0 as *const NMHDR);
                    if nmhdr.code == TCN_SELCHANGE {
                        let selected_index = SendMessageW(hwnd, TCM_GETCURSEL, None, None).0 as usize;
                        let id = tab_view_context.tab_ids.borrow().get(selected_index).cloned();
                        current_selected.set(id);
                    }
                },
                _ => (),
            }
        }),
    );

    element.on_destroy(closure!(
        [parent_context] || {
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
            app_state.remove_control_handler(hwnd);
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
        ContextProvider<TabViewContext>(
            .value = tab_view_context
        ) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    parent_hwnd: hwnd,
                    add_child: None,
                    remove_child: None,
                    parent_node: Some(node_id),
                },
            ) {
                $(props.children.clone())
            }
        }
    }
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_view_context = element.context::<TabViewContext>().unwrap();

    let mut tie = TCITEMW::default();

    let mut title: Vec<u16> = props.title.get().encode_utf16().collect();
    title.push(0);
    tie.mask = TCIF_TEXT;
    tie.pszText = PWSTR(title.as_mut_ptr() as *mut _);

    let index = unsafe { SendMessageW(parent_context.parent_hwnd, TCM_GETITEMCOUNT, None, None) };

    tab_view_context.tab_ids.borrow_mut().push(props.id.get());
    if tab_view_context.current_selected.borrow().is_none() {
        tab_view_context.current_selected.set(Some(props.id.get()));
    }

    unsafe {
        SendMessageW(
            parent_context.parent_hwnd,
            TCM_INSERTITEM,
            Some(WPARAM(index.0 as _)),
            Some(LPARAM(&tie as *const _ as _)),
        );
    }

    let subtree_context = Rc::new(TreeContext::new());
    let subtree_root = create_state(None);

    effect!(
        [tab_view_context.current_selected, props.id, subtree_root]
            || unsafe {
                if current_selected.get() == Some(id.get()) {
                    if let Some(subtree_root) = subtree_root.get() {
                        let _ = ShowWindow(subtree_root, SW_SHOW);
                    }
                } else {
                    if let Some(subtree_root) = subtree_root.get() {
                        let _ = ShowWindow(subtree_root, SW_HIDE);
                    }
                }
            }
    );

    effect!(
        [
            tree_context,
            subtree_context,
            parent_context.parent_node,
            parent_context.parent_hwnd,
            window_context.scale_factor,
            subtree_root,
        ] || {
            if let Some(parent_node) = parent_node {
                if tree_context.layout(parent_node).is_some() {
                    if let Some(subtree_root) = subtree_root.get() {
                        resize_tab_view_content(
                            &subtree_context,
                            scale_factor.get(),
                            parent_hwnd,
                            subtree_root,
                        );
                        subtree_context.refresh();
                    }
                }
            }
        }
    );

    layout! {
        ContextProvider<TreeContext>(.value = subtree_context.clone()) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    parent_hwnd: parent_context.parent_hwnd,
                    add_child: Some(callback!([window_context.scale_factor] |child_hwnd: HWND, child_node: Option<NodeId>| {
                        subtree_context.set_root_node(child_node);
                        subtree_root.set(Some(child_hwnd));

                        resize_tab_view_content(&subtree_context, scale_factor.get(), parent_context.parent_hwnd, child_hwnd);
                    })),
                    remove_child: None,
                    parent_node: None,
                },
            ) {
                $(props.children.get())
            }
        }
    }
}

fn resize_tab_view_content(
    tree_context: &TreeContext,
    scale_factor: f64,
    tab_control: HWND,
    content: HWND,
) {
    let mut tab_control_rect = RECT::default();
    unsafe {
        GetClientRect(tab_control, &mut tab_control_rect).unwrap();
    }

    let mut adjust_rect = RECT::default();
    unsafe {
        SendMessageW(
            tab_control,
            TCM_ADJUSTRECT,
            Some(WPARAM(false as _)),
            Some(LPARAM(&mut adjust_rect as *mut _ as _)),
        );
    }

    let client_rect = RECT {
        left: tab_control_rect.left + adjust_rect.left,
        top: tab_control_rect.top + adjust_rect.top,
        right: tab_control_rect.right + adjust_rect.right,
        bottom: tab_control_rect.bottom + adjust_rect.bottom,
    };
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    unsafe {
        SetWindowPos(
            content,
            None,
            client_rect.left,
            client_rect.top,
            width,
            height,
            SWP_NOZORDER,
        )
        .unwrap();
    }

    let size: LogicalSize<f32> = PhysicalSize::new(width, height).to_logical(scale_factor);
    if let Some(root_node) = tree_context.root_node() {
        tree_context.update_style(root_node, |prev| Style {
            size: Size {
                width: Dimension::from_length(size.width),
                height: Dimension::from_length(size.height),
            },
            ..prev
        });
    }
}
