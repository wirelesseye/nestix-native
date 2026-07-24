use std::{cell::RefCell, rc::Rc, sync::Once};

use nestix::{
    Element, Layout, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension as NativeDimension, StyleContext, StyleScope, TabViewItemProps, TabViewProps,
    TreeContext,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    matched_style, resolved_view_style, style_align_self, style_dimension, style_flex_basis,
    style_flex_grow, style_flex_shrink, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        UI::{
            Controls::{
                ICC_TAB_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, NMHDR, TCIF_TEXT,
                TCITEMW, TCM_ADJUSTRECT, TCM_DELETEITEM, TCM_GETCURSEL, TCM_GETITEMCOUNT,
                TCM_INSERTITEM, TCM_SETITEM, TCN_SELCHANGE, WC_TABCONTROL,
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
    pages: RefCell<Vec<(HWND, Rc<TreeContext>)>>,
}

#[component]
/// Renders a native Win32 tab view.
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabView", "__win32_TabView"];

    let app_state = element.context::<AppState>().unwrap();
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(style_props.clone(), &props.view);

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
    element.provide_handle(hwnd);

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [parent_context] | placement | {
            parent_context.place_child(hwnd, Some(node_id), placement);
        }
    ));

    let tab_view_context = Rc::new(TabViewContext {
        current_selected: current_selected.clone(),
        tab_ids: RefCell::new(Vec::new()),
        pages: RefCell::new(Vec::new()),
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

    element.on_unmount(closure!(
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

    scoped_effect!(
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

    scoped_effect!(
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
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
                NativeDimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                NativeDimension::Auto,
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
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left = style_dimension(
                style_props.as_ref(),
                left.get(),
                NativeDimension::Auto,
                |style| style.left,
            );
            let top = style_dimension(
                style_props.as_ref(),
                top.get(),
                NativeDimension::Auto,
                |style| style.top,
            );
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
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
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            tab_view_context,
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

                    // A tab page can be mounted while the tab control still has
                    // its initial zero-sized window. Size every registered page
                    // from the native control after the control receives its
                    // computed layout, so no page can miss this first update.
                    let pages = tab_view_context.pages.borrow().clone();
                    for (page_hwnd, page_tree_context) in pages {
                        resize_tab_view_content(&page_tree_context, scale_factor, hwnd, page_hwnd);
                    }
                }
            }
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<TabViewContext>(tab_view_context) {
                ContextProvider<ParentContext>(
                    ParentContext {
                        parent_hwnd: hwnd,
                        add_child: None,
                        insert_child: None,
                        remove_child: None,
                        parent_node: Some(node_id),
                    },
                ) {
                    $(props.children.clone())
                }
            }
        }
    }
}

#[component]
/// Renders one page and tab label within a [`TabView`].
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabViewItem", "__win32_TabViewItem"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_view_context = element.context::<TabViewContext>().unwrap();

    let subtree_context = Rc::new(TreeContext::new());
    let subtree_root = create_state(None);

    element.on_place(closure!(
        [parent_context, tab_view_context, props.id, props.title] | placement | {
            let id = id.get();
            let existing_index = tab_view_context
                .tab_ids
                .borrow()
                .iter()
                .position(|tab_id| *tab_id == id);

            if let Some(existing_index) = existing_index {
                tab_view_context.tab_ids.borrow_mut().remove(existing_index);
                unsafe {
                    SendMessageW(
                        parent_context.parent_hwnd,
                        TCM_DELETEITEM,
                        Some(WPARAM(existing_index)),
                        None,
                    );
                }
            }

            let index = placement
                .index
                .unwrap_or_else(|| unsafe {
                    SendMessageW(parent_context.parent_hwnd, TCM_GETITEMCOUNT, None, None).0
                        as usize
                })
                .min(tab_view_context.tab_ids.borrow().len());

            insert_tab_item(parent_context.parent_hwnd, index, &title.get());

            tab_view_context
                .tab_ids
                .borrow_mut()
                .insert(index, id.clone());
            if tab_view_context.current_selected.borrow().is_none() {
                tab_view_context.current_selected.set(Some(id));
            }
        }
    ));

    element.on_unmount(closure!(
        [parent_context, tab_view_context, props.id, subtree_root] || {
            let id = id.get();
            let existing_index = tab_view_context
                .tab_ids
                .borrow()
                .iter()
                .position(|tab_id| *tab_id == id);

            if let Some(existing_index) = existing_index {
                tab_view_context.tab_ids.borrow_mut().remove(existing_index);
                unsafe {
                    SendMessageW(
                        parent_context.parent_hwnd,
                        TCM_DELETEITEM,
                        Some(WPARAM(existing_index)),
                        None,
                    );
                }
            }

            if let Some(subtree_root) = subtree_root.get() {
                tab_view_context
                    .pages
                    .borrow_mut()
                    .retain(|(page_hwnd, _)| *page_hwnd != subtree_root);
            }
        }
    ));

    scoped_effect!(
        [
            parent_context.parent_hwnd,
            tab_view_context,
            props.id,
            props.title
        ] || {
            let id = id.get();
            let index = tab_view_context
                .tab_ids
                .borrow()
                .iter()
                .position(|tab_id| *tab_id == id);

            if let Some(index) = index {
                set_tab_item_title(parent_hwnd, index, &title.get());
            }
        }
    );

    scoped_effect!(
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

    scoped_effect!(
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
                    }
                }
            }
        }
    );

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<ParentContext>(
                    ParentContext {
                        parent_hwnd: parent_context.parent_hwnd,
                        add_child: Some(callback!([window_context.scale_factor, tab_view_context] |child_hwnd: HWND,
                        child_node: Option<NodeId> | {
                            subtree_context.set_root_node(child_node);
                            subtree_root.set(Some(child_hwnd));
                            let mut pages = tab_view_context.pages.borrow_mut();
                            pages.retain(|(page_hwnd, _)| *page_hwnd != child_hwnd);
                            pages.push((child_hwnd, subtree_context.clone()));
                            drop(pages);
                            resize_tab_view_content(
                                &subtree_context,
                                scale_factor.get(),
                                parent_context.parent_hwnd,
                                child_hwnd,
                            );
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

fn insert_tab_item(tab_control: HWND, index: usize, title: &str) {
    let (mut item, _title) = tab_item_with_title(title);
    unsafe {
        SendMessageW(
            tab_control,
            TCM_INSERTITEM,
            Some(WPARAM(index)),
            Some(LPARAM(&mut item as *mut _ as _)),
        );
    }
}

fn set_tab_item_title(tab_control: HWND, index: usize, title: &str) {
    let (mut item, _title) = tab_item_with_title(title);
    unsafe {
        SendMessageW(
            tab_control,
            TCM_SETITEM,
            Some(WPARAM(index)),
            Some(LPARAM(&mut item as *mut _ as _)),
        );
    }
}

fn tab_item_with_title(title: &str) -> (TCITEMW, Vec<u16>) {
    let mut title: Vec<u16> = title.encode_utf16().collect();
    title.push(0);

    let mut item = TCITEMW::default();
    item.mask = TCIF_TEXT;
    item.pszText = PWSTR(title.as_mut_ptr());
    (item, title)
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

    // TCM_ADJUSTRECT expects the tab control's window rectangle as input when
    // calculating the display rectangle. Passing an empty rectangle can leave
    // a newly-created page with an invalid size until the next window resize.
    let mut adjust_rect = tab_control_rect;
    unsafe {
        SendMessageW(
            tab_control,
            TCM_ADJUSTRECT,
            Some(WPARAM(false as _)),
            Some(LPARAM(&mut adjust_rect as *mut _ as _)),
        );
    }

    let width = adjust_rect.right - adjust_rect.left;
    let height = adjust_rect.bottom - adjust_rect.top;

    unsafe {
        SetWindowPos(
            content,
            None,
            adjust_rect.left,
            adjust_rect.top,
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
        // The root is registered during placement, after its initial style
        // effects may already have run. Refresh here so every tab page gets a
        // layout immediately instead of waiting for an external resize.
        tree_context.refresh();
    }
}
