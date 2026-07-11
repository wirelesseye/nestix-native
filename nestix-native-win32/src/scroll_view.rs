use std::{cell::RefCell, collections::HashMap, mem::size_of, rc::Rc, sync::Once};

use nestix::{
    Element, Layout, callback, closure, component, components::ContextProvider, layout,
    scoped_effect,
};
use nestix_native_core::{
    Dimension, ScrollViewProps, StyleContext, StyleScope, TreeContext,
    dpi::{LogicalPosition, LogicalSize},
    matched_style, style_align_self, style_dimension, style_grow, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{NodeId, Size, Style, style_helpers::FromLength};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
            IDC_ARROW, LoadCursorW, RegisterClassW, SB_BOTTOM, SB_ENDSCROLL, SB_HORZ, SB_LINEDOWN,
            SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SB_VERT,
            SCROLLBAR_CONSTANTS, SCROLLINFO, SIF_ALL, SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS,
            SWP_NOZORDER, SetWindowPos, WINDOW_EX_STYLE, WM_HSCROLL, WM_MOUSEHWHEEL, WM_MOUSEWHEEL,
            WM_SIZE, WM_VSCROLL, WNDCLASSW, WS_CHILD, WS_HSCROLL, WS_VISIBLE, WS_VSCROLL,
        },
    },
    core::{BOOL, PCWSTR, w},
};

use crate::{WindowContext, contexts::ParentContext};

#[link(name = "user32")]
unsafe extern "system" {
    fn SetScrollInfo(
        hwnd: HWND,
        nbar: SCROLLBAR_CONSTANTS,
        info: *const SCROLLINFO,
        redraw: BOOL,
    ) -> i32;
    fn ShowScrollBar(hwnd: HWND, nbar: SCROLLBAR_CONSTANTS, show: BOOL) -> BOOL;
}

#[derive(Clone)]
struct ScrollState {
    content: HWND,
    content_width: i32,
    content_height: i32,
    viewport_width: i32,
    viewport_height: i32,
    x: i32,
    y: i32,
    scroll_x: bool,
    scroll_y: bool,
    visible_x: bool,
    visible_y: bool,
    subtree: Rc<TreeContext>,
    subtree_root: NodeId,
    scale_factor: f64,
    applied_viewport_width: i32,
    applied_viewport_height: i32,
}

thread_local! {
    static SCROLL_STATES: RefCell<HashMap<*mut std::ffi::c_void, ScrollState>> =
        RefCell::new(HashMap::new());
}

fn window_classname(hinstance: HINSTANCE) -> PCWSTR {
    const CLASSNAME: PCWSTR = w!("NestixNativeScrollView");
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        RegisterClassW(&WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hInstance: hinstance,
            lpszClassName: CLASSNAME,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        });
    });
    CLASSNAME
}

#[component]
pub fn ScrollView(props: &ScrollViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__ScrollView", "__win32_ScrollView"];

    let window = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let child_nodes = Rc::new(RefCell::new(Vec::new()));
    let subtree = Rc::new(TreeContext::new());
    let subtree_root = subtree.create_node(false);
    subtree.set_root_node(Some(subtree_root));
    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_classname(hinstance.into()),
            None,
            WS_CHILD | WS_VISIBLE | WS_HSCROLL | WS_VSCROLL,
            0,
            0,
            0,
            0,
            Some(parent.parent_hwnd),
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };
    let content = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            None,
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            Some(hwnd),
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };
    unsafe {
        let _ = ShowScrollBar(hwnd, SB_HORZ, false.into());
        let _ = ShowScrollBar(hwnd, SB_VERT, false.into());
    }
    SCROLL_STATES.with_borrow_mut(|states| {
        states.insert(
            hwnd.0,
            ScrollState {
                content,
                content_width: 0,
                content_height: 0,
                viewport_width: 0,
                viewport_height: 0,
                x: 0,
                y: 0,
                scroll_x: props.scroll_x.get(),
                scroll_y: props.scroll_y.get(),
                visible_x: false,
                visible_y: false,
                subtree: subtree.clone(),
                subtree_root,
                scale_factor: window.scale_factor.get(),
                applied_viewport_width: -1,
                applied_viewport_height: -1,
            },
        );
    });
    element.provide_handle(hwnd);
    let node = tree_context.create_node(false);

    element.on_place(closure!(
        [parent] | placement | {
            if let Some(index) = placement.index
                && let Some(insert) = &parent.insert_child
            {
                insert(hwnd, Some(node), index);
            } else if let Some(add) = &parent.add_child {
                add(hwnd, Some(node));
            }
        }
    ));
    element.on_unmount(closure!(
        [parent] || {
            SCROLL_STATES.with_borrow_mut(|states| states.remove(&hwnd.0));
            unsafe { DestroyWindow(hwnd).unwrap() };
            if let Some(remove) = &parent.remove_child {
                remove(hwnd, Some(node));
            }
        }
    ));

    scoped_effect!(
        element,
        [props.scroll_x, props.scroll_y] || {
            SCROLL_STATES.with_borrow_mut(|states| {
                if let Some(state) = states.get_mut(&hwnd.0) {
                    state.scroll_x = scroll_x.get();
                    state.scroll_y = scroll_y.get();
                    update_scrollbars(hwnd, state);
                }
            });
            sync_subtree_viewport(hwnd);
        }
    );

    scoped_effect!(
        element,
        [tree_context, styles, props.view.grow, props.view.align_self] || {
            let style = styles.get();
            tree_context.update_style(node, |prev| Style {
                flex_grow: style_grow(style.as_ref(), grow.get()),
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window.scale_factor,
            tree_context,
            styles,
            props.view.width,
            props.view.height,
            props.view.left,
            props.view.top,
            props.view.margin()
        ] || {
            let scale = scale_factor.get();
            let style = styles.get();
            let width = style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width);
            let height =
                style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| s.height);
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node, |prev| Style {
                flex_direction: taffy::FlexDirection::Column,
                size: Size {
                    width: width.to_taffy(scale),
                    height: height.to_taffy(scale),
                },
                inset: inset_to_taffy(left, top, scale),
                margin: margin_to_taffy(style_margin(style.as_ref(), margin.get()), scale),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [window.scale_factor, tree_context, parent.parent_node] || {
            if parent_node.is_some()
                && let Some(value) = tree_context.layout(node)
            {
                let scale = scale_factor.get();
                let point =
                    LogicalPosition::new(value.location.x, value.location.y).to_physical(scale);
                let size = LogicalSize::new(value.size.width, value.size.height).to_physical(scale);
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
                    .unwrap()
                };
                SCROLL_STATES.with_borrow_mut(|states| {
                    if let Some(state) = states.get_mut(&hwnd.0) {
                        state.scale_factor = scale;
                        update_scrollbars(hwnd, state);
                    }
                });
                sync_subtree_viewport(hwnd);
            }
        }
    );

    scoped_effect!(
        element,
        [window.scale_factor, subtree] || {
            if let Some(value) = subtree.layout(subtree_root) {
                let size = LogicalSize::new(value.size.width, value.size.height)
                    .to_physical(scale_factor.get());
                SCROLL_STATES.with_borrow_mut(|states| {
                    if let Some(state) = states.get_mut(&hwnd.0) {
                        state.content_width = size.width;
                        state.content_height = size.height;
                        update_scrollbars(hwnd, state);
                    }
                });
                sync_subtree_viewport(hwnd);
            }
        }
    );

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TreeContext>(subtree.clone()) {
                ContextProvider<ParentContext>(ParentContext {
                    parent_hwnd: content,
                    add_child: Some(callback!([subtree, child_nodes] |_: HWND, child_node: Option<NodeId>| {
                        if let Some(child_node) = child_node {
                            if child_nodes.borrow().contains(&child_node) {
                                subtree.remove_child(subtree_root, child_node);
                                child_nodes.borrow_mut().retain(|node| *node != child_node);
                            }
                            subtree.add_child(subtree_root, child_node);
                            child_nodes.borrow_mut().push(child_node);
                            subtree.refresh();
                        }
                    })),
                    insert_child: Some(callback!([subtree, child_nodes] |_: HWND, child_node: Option<NodeId>, index: usize| {
                        if let Some(child_node) = child_node {
                            if child_nodes.borrow().contains(&child_node) {
                                subtree.remove_child(subtree_root, child_node);
                                child_nodes.borrow_mut().retain(|node| *node != child_node);
                            }
                            let index = index.min(child_nodes.borrow().len());
                            subtree.insert_child(subtree_root, child_node, index);
                            child_nodes.borrow_mut().insert(index, child_node);
                            subtree.refresh();
                        }
                    })),
                    remove_child: Some(callback!([subtree, child_nodes] |_: HWND, child_node: Option<NodeId>| {
                        if let Some(child_node) = child_node {
                            child_nodes.borrow_mut().retain(|node| *node != child_node);
                            subtree.remove_child(subtree_root, child_node);
                            subtree.refresh();
                        }
                    })),
                    parent_node: Some(subtree_root),
                }) {
                    $(props.children.clone().map(|element| Layout::from(element.clone())))
                }
            }
        }
    }
}

fn update_scrollbars(hwnd: HWND, state: &mut ScrollState) {
    if !state.scroll_x {
        state.x = 0;
    }
    if !state.scroll_y {
        state.y = 0;
    }
    for _ in 0..3 {
        let mut rect = Default::default();
        unsafe { GetClientRect(hwnd, &mut rect).unwrap() };
        state.viewport_width = rect.right - rect.left;
        state.viewport_height = rect.bottom - rect.top;
        let visible_x = state.scroll_x && state.content_width > state.viewport_width;
        let visible_y = state.scroll_y && state.content_height > state.viewport_height;
        if visible_x == state.visible_x && visible_y == state.visible_y {
            break;
        }
        state.visible_x = visible_x;
        state.visible_y = visible_y;
        unsafe {
            let _ = ShowScrollBar(hwnd, SB_HORZ, visible_x.into());
            let _ = ShowScrollBar(hwnd, SB_VERT, visible_y.into());
        }
    }
    state.x = state
        .x
        .clamp(0, (state.content_width - state.viewport_width).max(0));
    state.y = state
        .y
        .clamp(0, (state.content_height - state.viewport_height).max(0));
    unsafe {
        set_scroll_info(
            hwnd,
            SB_HORZ,
            state.content_width,
            state.viewport_width,
            state.x,
        );
        set_scroll_info(
            hwnd,
            SB_VERT,
            state.content_height,
            state.viewport_height,
            state.y,
        );
        SetWindowPos(
            state.content,
            None,
            -state.x,
            -state.y,
            state.content_width.max(state.viewport_width),
            state.content_height.max(state.viewport_height),
            SWP_NOZORDER,
        )
        .unwrap();
    }
}

fn sync_subtree_viewport(hwnd: HWND) {
    let update = SCROLL_STATES.with_borrow_mut(|states| {
        let state = states.get_mut(&hwnd.0)?;
        if state.viewport_width == state.applied_viewport_width
            && state.viewport_height == state.applied_viewport_height
        {
            return None;
        }
        state.applied_viewport_width = state.viewport_width;
        state.applied_viewport_height = state.viewport_height;
        Some((
            state.subtree.clone(),
            state.subtree_root,
            state.viewport_width as f32 / state.scale_factor as f32,
            state.viewport_height as f32 / state.scale_factor as f32,
        ))
    });
    if let Some((subtree, root, width, height)) = update {
        subtree.update_style(root, |prev| Style {
            min_size: Size {
                width: taffy::Dimension::from_length(width.max(0.0)),
                height: taffy::Dimension::from_length(height.max(0.0)),
            },
            ..prev
        });
        subtree.refresh();
    }
}

unsafe fn set_scroll_info(hwnd: HWND, bar: SCROLLBAR_CONSTANTS, extent: i32, page: i32, pos: i32) {
    let info = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: extent.saturating_sub(1),
        nPage: page.max(0) as u32,
        nPos: pos,
        ..Default::default()
    };
    unsafe { SetScrollInfo(hwnd, bar, &info, true.into()) };
}

fn scroll_command(hwnd: HWND, bar: SCROLLBAR_CONSTANTS, command: u16) {
    SCROLL_STATES.with_borrow_mut(|states| {
        let Some(state) = states.get_mut(&hwnd.0) else {
            return;
        };
        let (position, page, max) = if bar == SB_HORZ {
            (
                &mut state.x,
                state.viewport_width,
                (state.content_width - state.viewport_width).max(0),
            )
        } else {
            (
                &mut state.y,
                state.viewport_height,
                (state.content_height - state.viewport_height).max(0),
            )
        };
        *position = match command as i32 {
            value if value == SB_LINEUP.0 => *position - 24,
            value if value == SB_LINEDOWN.0 => *position + 24,
            value if value == SB_PAGEUP.0 => *position - page,
            value if value == SB_PAGEDOWN.0 => *position + page,
            value if value == SB_TOP.0 => 0,
            value if value == SB_BOTTOM.0 => max,
            value if value == SB_THUMBTRACK.0 || value == SB_THUMBPOSITION.0 => {
                let mut info = SCROLLINFO {
                    cbSize: size_of::<SCROLLINFO>() as u32,
                    fMask: SIF_ALL | SIF_TRACKPOS,
                    ..Default::default()
                };
                unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetScrollInfo(hwnd, bar, &mut info)
                        .unwrap()
                };
                info.nTrackPos
            }
            value if value == SB_ENDSCROLL.0 => *position,
            _ => *position,
        }
        .clamp(0, max);
        update_scrollbars(hwnd, state);
    });
    sync_subtree_viewport(hwnd);
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            let updated = SCROLL_STATES.with(|states| {
                if let Ok(mut states) = states.try_borrow_mut()
                    && let Some(state) = states.get_mut(&hwnd.0)
                {
                    update_scrollbars(hwnd, state);
                    true
                } else {
                    false
                }
            });
            if updated {
                sync_subtree_viewport(hwnd);
            }
            LRESULT(0)
        }
        WM_HSCROLL => {
            scroll_command(hwnd, SB_HORZ, wparam.0 as u16);
            LRESULT(0)
        }
        WM_VSCROLL => {
            scroll_command(hwnd, SB_VERT, wparam.0 as u16);
            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let delta = ((wparam.0 >> 16) as u16) as i16 as i32;
            SCROLL_STATES.with_borrow_mut(|states| {
                if let Some(state) = states.get_mut(&hwnd.0) {
                    if msg == WM_MOUSEHWHEEL && state.scroll_x {
                        state.x -= delta / 4;
                    } else if state.scroll_y {
                        state.y -= delta / 4;
                    } else if state.scroll_x {
                        state.x -= delta / 4;
                    }
                    update_scrollbars(hwnd, state);
                }
            });
            sync_subtree_viewport(hwnd);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
