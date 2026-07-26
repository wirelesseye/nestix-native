use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::c_void,
    rc::{Rc, Weak},
    sync::Once,
};

use nestix::Shared;
use nestix_native_core::{
    Color, TreeContext,
    dpi::{LogicalPosition, LogicalSize},
};
use taffy::{NodeId, Size, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, COLOR_BTNFACE, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
            GetSysColor, GetSysColorBrush, HBRUSH, InvalidateRect, OPAQUE, PAINTSTRUCT, SetBkColor,
            SetBkMode, SetTextColor, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::{DRAWITEMSTRUCT, NMHDR},
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                BeginDeferWindowPos, CreateWindowExW, DefWindowProcW, DeferWindowPos,
                EndDeferWindowPos, GWL_STYLE, GetClientRect, GetWindowLongPtrW, HWND_TOP,
                IDC_ARROW, LoadCursorW, PostMessageW, RegisterClassW, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WINDOW_EX_STYLE, WM_APP,
                WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORSTATIC, WM_DRAWITEM, WM_ERASEBKGND,
                WM_HSCROLL, WM_NOTIFY, WM_PAINT, WM_VSCROLL, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN,
                WS_CLIPSIBLINGS, WS_VISIBLE,
            },
        },
    },
    core::{PCWSTR, w},
};

use crate::{font::colorref, shared_app_state};

pub(crate) type VisualId = usize;

#[derive(Clone)]
pub(crate) struct VisualHandle {
    surface: Rc<VisualSurface>,
    id: VisualId,
}

impl VisualHandle {
    pub(crate) fn surface(&self) -> &Rc<VisualSurface> {
        &self.surface
    }

    pub(crate) fn id(&self) -> VisualId {
        self.id
    }

    pub(crate) fn hwnd(&self) -> HWND {
        self.surface.hwnd
    }

    pub(crate) fn contains_client_point(&self, point: windows::Win32::Foundation::POINT) -> bool {
        self.surface
            .nodes
            .borrow()
            .get(&self.id)
            .is_some_and(|node| {
                point.x >= node.rect.left
                    && point.x < node.rect.right
                    && point.y >= node.rect.top
                    && point.y < node.rect.bottom
            })
    }

    pub(crate) fn relative_logical_position(
        &self,
        point: windows::Win32::Foundation::POINT,
        scale: f64,
    ) -> nestix_native_core::dpi::LogicalPosition<f64> {
        let rect = self
            .surface
            .nodes
            .borrow()
            .get(&self.id)
            .map(|node| node.rect)
            .unwrap_or_default();
        nestix_native_core::dpi::LogicalPosition::new(
            (point.x - rect.left) as f64 / scale,
            (point.y - rect.top) as f64 / scale,
        )
    }
}

struct VisualNode {
    parent: Option<VisualId>,
    children: Vec<VisualId>,
    layout_node: NodeId,
    hwnd: Option<HWND>,
    rect: RECT,
    background_color: Option<Color>,
    background: Option<HBRUSH>,
}

const WM_NESTIX_SYNC_SURFACE: u32 = WM_APP + 0x58;

pub(crate) struct VisualSurface {
    hwnd: HWND,
    tree: Rc<TreeContext>,
    container_node: Option<NodeId>,
    nodes: RefCell<HashMap<VisualId, VisualNode>>,
    roots: RefCell<Vec<VisualId>>,
    hwnd_nodes: RefCell<HashMap<*mut c_void, VisualId>>,
    next_id: Cell<VisualId>,
    sync_pending: Cell<bool>,
    sync_scale_factor: Cell<f64>,
    native_order_dirty: Cell<bool>,
    after_sync: RefCell<HashMap<usize, Rc<dyn Fn(f64)>>>,
    next_after_sync_id: Cell<usize>,
}

thread_local! {
    static SURFACES: RefCell<HashMap<*mut c_void, Weak<VisualSurface>>> = RefCell::new(HashMap::new());
}

impl VisualSurface {
    pub(crate) fn new(
        hwnd: HWND,
        tree: Rc<TreeContext>,
        container_node: Option<NodeId>,
    ) -> Rc<Self> {
        tree.set_defer_refreshes(true);
        let surface = Rc::new(Self {
            hwnd,
            tree,
            container_node,
            nodes: RefCell::new(HashMap::new()),
            roots: RefCell::new(Vec::new()),
            hwnd_nodes: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
            sync_pending: Cell::new(false),
            sync_scale_factor: Cell::new(1.0),
            native_order_dirty: Cell::new(true),
            after_sync: RefCell::new(HashMap::new()),
            next_after_sync_id: Cell::new(0),
        });
        SURFACES.with_borrow_mut(|surfaces| {
            surfaces.insert(hwnd.0, Rc::downgrade(&surface));
        });
        surface
    }

    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub(crate) fn mount(
        self: &Rc<Self>,
        parent: Option<VisualId>,
        layout_node: NodeId,
        hwnd: Option<HWND>,
    ) -> VisualHandle {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.nodes.borrow_mut().insert(
            id,
            VisualNode {
                parent,
                children: Vec::new(),
                layout_node,
                hwnd,
                rect: RECT::default(),
                background_color: None,
                background: None,
            },
        );
        if let Some(hwnd) = hwnd {
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                SetWindowLongPtrW(hwnd, GWL_STYLE, (style | WS_CLIPSIBLINGS.0) as isize);
            }
            self.hwnd_nodes.borrow_mut().insert(hwnd.0, id);
        }
        self.children_mut(parent).push(id);
        self.native_order_dirty.set(true);
        self.update_layout_children(parent);
        self.tree.refresh();
        VisualHandle {
            surface: self.clone(),
            id,
        }
    }

    pub(crate) fn place(&self, id: VisualId, predecessor: Option<VisualId>) {
        let parent = self.nodes.borrow().get(&id).and_then(|node| node.parent);
        let predecessor =
            predecessor.and_then(|predecessor| self.direct_child(parent, predecessor));
        let mut children = self.children_mut(parent);
        children.retain(|child| *child != id);
        let index = predecessor
            .and_then(|predecessor| {
                children
                    .iter()
                    .position(|child| *child == predecessor)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        let index = index.min(children.len());
        children.insert(index, id);
        drop(children);
        self.native_order_dirty.set(true);
        self.update_layout_children(parent);
        self.tree.refresh();
    }

    pub(crate) fn remove(&self, id: VisualId) {
        let Some(node) = self.nodes.borrow_mut().remove(&id) else {
            return;
        };
        self.children_mut(node.parent).retain(|child| *child != id);
        if let Some(hwnd) = node.hwnd {
            self.hwnd_nodes.borrow_mut().remove(&hwnd.0);
        }
        self.native_order_dirty.set(true);
        if let Some(brush) = node.background {
            unsafe {
                let _ = DeleteObject(brush.into());
            }
        }
        self.update_layout_children(node.parent);
        self.tree.refresh();
        self.invalidate_rect(node.rect);
    }

    pub(crate) fn set_background(&self, id: VisualId, color: Option<Color>) {
        let mut nodes = self.nodes.borrow_mut();
        let Some(node) = nodes.get_mut(&id) else {
            return;
        };
        if node.background_color == color {
            return;
        }
        node.background_color = color;
        if let Some(brush) = node.background.take() {
            unsafe {
                let _ = DeleteObject(brush.into());
            }
        }
        node.background = color.and_then(|color| {
            let rgb = color.into_rgb();
            (rgb.alpha > 0).then(|| unsafe {
                CreateSolidBrush(COLORREF(
                    rgb.red as u32 | ((rgb.green as u32) << 8) | ((rgb.blue as u32) << 16),
                ))
            })
        });
        let rect = node.rect;
        drop(nodes);
        self.invalidate_rect(rect);
    }

    pub(crate) fn resolve_handle(&self, handle: &Shared<dyn std::any::Any>) -> Option<VisualId> {
        if let Some(handle) = handle.downcast_ref::<VisualHandle>()
            && Rc::as_ptr(&handle.surface) == self as *const Self
        {
            return Some(handle.id);
        }
        handle
            .downcast_ref::<HWND>()
            .and_then(|hwnd| self.hwnd_nodes.borrow().get(&hwnd.0).copied())
    }

    pub(crate) fn schedule_sync(&self, scale_factor: f64) {
        self.sync_scale_factor.set(scale_factor);
        if self.sync_pending.replace(true) {
            return;
        }
        if unsafe {
            PostMessageW(
                Some(self.hwnd),
                WM_NESTIX_SYNC_SURFACE,
                WPARAM(0),
                LPARAM(0),
            )
        }
        .is_err()
        {
            self.sync_pending.set(false);
        }
    }

    pub(crate) fn add_after_sync(&self, callback: impl Fn(f64) + 'static) -> usize {
        let id = self.next_after_sync_id.get();
        self.next_after_sync_id.set(id + 1);
        self.after_sync.borrow_mut().insert(id, Rc::new(callback));
        id
    }

    pub(crate) fn remove_after_sync(&self, id: usize) {
        self.after_sync.borrow_mut().remove(&id);
    }

    fn run_scheduled_sync(&self) {
        self.sync_pending.set(false);
        self.tree.flush_refresh();
        self.sync(self.sync_scale_factor.get());
    }

    fn sync(&self, scale_factor: f64) {
        let roots = self.roots.borrow().clone();
        let mut dirty_rect = None;
        let mut native_positions = Vec::new();
        for root in roots {
            dirty_rect = union_optional_rects(
                dirty_rect,
                self.sync_node(
                    root,
                    LogicalPosition::new(0.0, 0.0),
                    scale_factor,
                    &mut native_positions,
                ),
            );
        }
        self.apply_native_positions(&native_positions);
        if self.native_order_dirty.replace(false) {
            let mut native = Vec::new();
            for root in self.roots.borrow().iter().copied() {
                self.collect_native(root, &mut native);
            }
            let mut insert_after = HWND_TOP;
            for hwnd in native.into_iter().rev() {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(insert_after),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                insert_after = hwnd;
            }
        }
        if let Some(rect) = dirty_rect {
            self.invalidate_rect(rect);
        }
        let callbacks = self
            .after_sync
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for callback in callbacks {
            callback(scale_factor);
        }
    }

    fn collect_native(&self, id: VisualId, native: &mut Vec<HWND>) {
        let (hwnd, children) = {
            let nodes = self.nodes.borrow();
            let Some(node) = nodes.get(&id) else {
                return;
            };
            (node.hwnd, node.children.clone())
        };
        if let Some(hwnd) = hwnd {
            native.push(hwnd);
        }
        for child in children {
            self.collect_native(child, native);
        }
    }

    fn sync_node(
        &self,
        id: VisualId,
        origin: LogicalPosition<f32>,
        scale_factor: f64,
        native_positions: &mut Vec<(HWND, RECT)>,
    ) -> Option<RECT> {
        let (layout_node, hwnd, children) = {
            let nodes = self.nodes.borrow();
            let Some(node) = nodes.get(&id) else {
                return None;
            };
            (node.layout_node, node.hwnd, node.children.clone())
        };
        let Some(layout) = self.tree.layout(layout_node) else {
            return None;
        };
        let location =
            LogicalPosition::new(origin.x + layout.location.x, origin.y + layout.location.y);
        let point = location.to_physical(scale_factor);
        let size: nestix_native_core::dpi::PhysicalSize<i32> =
            LogicalSize::new(layout.size.width, layout.size.height).to_physical(scale_factor);
        let rect = RECT {
            left: point.x,
            top: point.y,
            right: point.x + size.width,
            bottom: point.y + size.height,
        };
        let old_rect = self.nodes.borrow_mut().get_mut(&id).and_then(|node| {
            let changed = (node.rect != rect).then_some(node.rect);
            node.rect = rect;
            changed
        });
        if old_rect.is_some()
            && let Some(hwnd) = hwnd
        {
            native_positions.push((hwnd, rect));
        }
        let mut dirty_rect = old_rect.map(|old_rect| union_rects(old_rect, rect));
        for child in children {
            dirty_rect = union_optional_rects(
                dirty_rect,
                self.sync_node(child, location, scale_factor, native_positions),
            );
        }
        dirty_rect
    }

    fn apply_native_positions(&self, positions: &[(HWND, RECT)]) {
        if positions.is_empty() {
            return;
        }
        unsafe {
            if let Ok(mut batch) = BeginDeferWindowPos(positions.len() as i32) {
                let mut failed = false;
                for (hwnd, rect) in positions {
                    match DeferWindowPos(
                        batch,
                        *hwnd,
                        None,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    ) {
                        Ok(next) => batch = next,
                        Err(_) => {
                            failed = true;
                            break;
                        }
                    }
                }
                if !failed && EndDeferWindowPos(batch).is_ok() {
                    return;
                }
            }
            for (hwnd, rect) in positions {
                let _ = SetWindowPos(
                    *hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }

    fn direct_child(&self, parent: Option<VisualId>, mut id: VisualId) -> Option<VisualId> {
        loop {
            let node_parent = self.nodes.borrow().get(&id)?.parent;
            if node_parent == parent {
                return Some(id);
            }
            id = node_parent?;
        }
    }

    fn children_mut(&self, parent: Option<VisualId>) -> std::cell::RefMut<'_, Vec<VisualId>> {
        if let Some(parent) = parent {
            std::cell::RefMut::map(self.nodes.borrow_mut(), |nodes| {
                &mut nodes
                    .get_mut(&parent)
                    .expect("visual parent must exist")
                    .children
            })
        } else {
            self.roots.borrow_mut()
        }
    }

    fn update_layout_children(&self, parent: Option<VisualId>) {
        let children = if let Some(parent) = parent {
            let nodes = self.nodes.borrow();
            let Some(parent_node) = nodes.get(&parent) else {
                return;
            };
            let children = parent_node
                .children
                .iter()
                .filter_map(|child| nodes.get(child).map(|node| node.layout_node))
                .collect::<Vec<_>>();
            self.tree.set_children(parent_node.layout_node, &children);
            return;
        } else {
            let nodes = self.nodes.borrow();
            self.roots
                .borrow()
                .iter()
                .filter_map(|child| nodes.get(child).map(|node| node.layout_node))
                .collect::<Vec<_>>()
        };
        if let Some(container) = self.container_node {
            self.tree.set_children(container, &children);
        } else {
            let root = children.first().copied();
            self.tree.set_root_node(root);
            if let Some(root) = root {
                let mut rect = RECT::default();
                unsafe {
                    let _ = GetClientRect(self.hwnd, &mut rect);
                }
                let scale = unsafe { GetDpiForWindow(self.hwnd) }.max(96) as f32 / 96.0;
                self.tree.update_style(root, |prev| taffy::Style {
                    size: Size {
                        width: taffy::Dimension::from_length(
                            (rect.right - rect.left) as f32 / scale,
                        ),
                        height: taffy::Dimension::from_length(
                            (rect.bottom - rect.top) as f32 / scale,
                        ),
                    },
                    ..prev
                });
            }
        }
    }

    fn invalidate_rect(&self, rect: RECT) {
        if rect.left >= rect.right || rect.top >= rect.bottom {
            return;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), Some(&rect), false);
        }
    }

    fn paint(&self) {
        unsafe {
            let mut paint = PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut paint);
            FillRect(hdc, &paint.rcPaint, GetSysColorBrush(COLOR_BTNFACE));
            let roots = self.roots.borrow().clone();
            for root in roots {
                self.paint_node(hdc, root);
            }
            let _ = EndPaint(self.hwnd, &paint);
        }
    }

    fn paint_node(&self, hdc: windows::Win32::Graphics::Gdi::HDC, id: VisualId) {
        let (rect, brush, children) = {
            let nodes = self.nodes.borrow();
            let Some(node) = nodes.get(&id) else {
                return;
            };
            (node.rect, node.background, node.children.clone())
        };
        if let Some(brush) = brush {
            unsafe {
                FillRect(hdc, &rect, brush);
            }
        }
        for child in children {
            self.paint_node(hdc, child);
        }
    }

    fn control_background(&self, hwnd: HWND) -> HBRUSH {
        let mut id = self.hwnd_nodes.borrow().get(&hwnd.0).copied();
        while let Some(current) = id {
            let nodes = self.nodes.borrow();
            let Some(node) = nodes.get(&current) else {
                break;
            };
            if let Some(brush) = node.background {
                return brush;
            }
            id = node.parent;
        }
        unsafe { GetSysColorBrush(COLOR_BTNFACE) }
    }

    fn control_background_color(&self, hwnd: HWND) -> COLORREF {
        let mut id = self.hwnd_nodes.borrow().get(&hwnd.0).copied();
        while let Some(current) = id {
            let nodes = self.nodes.borrow();
            let Some(node) = nodes.get(&current) else {
                break;
            };
            if let Some(color) = node.background_color {
                let rgb = color.into_rgb();
                if rgb.alpha > 0 {
                    return COLORREF(
                        rgb.red as u32 | ((rgb.green as u32) << 8) | ((rgb.blue as u32) << 16),
                    );
                }
            }
            id = node.parent;
        }
        unsafe { COLORREF(GetSysColor(COLOR_BTNFACE)) }
    }
}

fn union_rects(first: RECT, second: RECT) -> RECT {
    RECT {
        left: first.left.min(second.left),
        top: first.top.min(second.top),
        right: first.right.max(second.right),
        bottom: first.bottom.max(second.bottom),
    }
}

fn union_optional_rects(first: Option<RECT>, second: Option<RECT>) -> Option<RECT> {
    match (first, second) {
        (Some(first), Some(second)) => Some(union_rects(first, second)),
        (Some(rect), None) | (None, Some(rect)) => Some(rect),
        (None, None) => None,
    }
}

impl Drop for VisualSurface {
    fn drop(&mut self) {
        SURFACES.with_borrow_mut(|surfaces| {
            let is_current = surfaces
                .get(&self.hwnd.0)
                .and_then(Weak::upgrade)
                .is_some_and(|surface| Rc::as_ptr(&surface) == self as *const Self);
            if is_current {
                surfaces.remove(&self.hwnd.0);
            }
        });
        for node in self.nodes.get_mut().values_mut() {
            if let Some(brush) = node.background.take() {
                unsafe {
                    let _ = DeleteObject(brush.into());
                }
            }
        }
    }
}

pub(crate) fn surface_for_hwnd(hwnd: HWND) -> Option<Rc<VisualSurface>> {
    SURFACES.with_borrow(|surfaces| surfaces.get(&hwnd.0).and_then(Weak::upgrade))
}

pub(crate) fn visual_handle(handle: &Shared<dyn std::any::Any>) -> Option<VisualHandle> {
    if let Some(handle) = handle.downcast_ref::<VisualHandle>() {
        return Some(handle.clone());
    }
    let hwnd = handle.downcast_ref::<HWND>()?;
    SURFACES.with_borrow(|surfaces| {
        surfaces.values().find_map(|surface| {
            let surface = surface.upgrade()?;
            let id = surface.hwnd_nodes.borrow().get(&hwnd.0).copied()?;
            Some(VisualHandle { surface, id })
        })
    })
}

pub(crate) fn handle_surface_message(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    let surface = surface_for_hwnd(hwnd)?;
    unsafe {
        match msg {
            WM_NESTIX_SYNC_SURFACE => {
                surface.run_scheduled_sync();
                Some(LRESULT(0))
            }
            WM_ERASEBKGND => Some(LRESULT(1)),
            WM_PAINT => {
                surface.paint();
                Some(LRESULT(0))
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                let app_state = shared_app_state();
                let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as _);
                let control = HWND(lparam.0 as _);
                if msg == WM_CTLCOLORSTATIC {
                    SetBkMode(hdc, OPAQUE);
                    SetBkColor(hdc, surface.control_background_color(control));
                } else {
                    SetBkMode(hdc, TRANSPARENT);
                }
                if let Some(color) = app_state.control_text_color(control) {
                    SetTextColor(hdc, colorref(color));
                }
                Some(LRESULT(surface.control_background(control).0 as isize))
            }
            WM_NOTIFY => {
                let header = &*(lparam.0 as *const NMHDR);
                shared_app_state().handle_control_event(header.hwndFrom, msg, wparam, lparam);
                Some(DefWindowProcW(hwnd, msg, wparam, lparam))
            }
            WM_COMMAND => {
                let control = HWND(lparam.0 as _);
                if control.0.is_null() {
                    None
                } else {
                    shared_app_state().handle_control_event(control, msg, wparam, lparam);
                    Some(DefWindowProcW(hwnd, msg, wparam, lparam))
                }
            }
            WM_HSCROLL | WM_VSCROLL => {
                let control = HWND(lparam.0 as _);
                if control.0.is_null() {
                    None
                } else {
                    shared_app_state().handle_control_event(control, msg, wparam, lparam);
                    Some(DefWindowProcW(hwnd, msg, wparam, lparam))
                }
            }
            WM_DRAWITEM => {
                let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
                shared_app_state().handle_control_event(item.hwndItem, msg, wparam, lparam);
                Some(LRESULT(1))
            }
            _ => None,
        }
    }
}

fn surface_classname(hinstance: HINSTANCE) -> PCWSTR {
    const CLASSNAME: PCWSTR = w!("NestixNativeSurface");
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        RegisterClassW(&WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hInstance: hinstance,
            lpszClassName: CLASSNAME,
            lpfnWndProc: Some(surface_window_proc),
            ..Default::default()
        });
    });
    CLASSNAME
}

pub(crate) fn create_child_surface(parent: HWND) -> windows::core::Result<HWND> {
    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            surface_classname(hinstance.into()),
            None,
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            0,
            0,
            0,
            0,
            Some(parent),
            None,
            Some(hinstance.into()),
            None,
        )
    }
}

extern "system" fn surface_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    handle_surface_message(hwnd, msg, wparam, lparam)
        .unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_resolves_descendants_to_direct_children() {
        let tree = Rc::new(TreeContext::new());
        let surface = VisualSurface::new(HWND::default(), tree.clone(), None);
        let parent = surface.mount(None, tree.create_node(false), None);
        let first = surface.mount(Some(parent.id()), tree.create_node(false), None);
        let descendant = surface.mount(Some(first.id()), tree.create_node(true), None);
        let second = surface.mount(Some(parent.id()), tree.create_node(true), None);

        surface.place(second.id(), Some(descendant.id()));

        assert_eq!(
            surface.nodes.borrow().get(&parent.id()).unwrap().children,
            vec![first.id(), second.id()]
        );
    }

    #[test]
    fn visual_hit_testing_uses_host_relative_bounds() {
        let tree = Rc::new(TreeContext::new());
        let surface = VisualSurface::new(HWND::default(), tree.clone(), None);
        let visual = surface.mount(None, tree.create_node(false), None);
        surface
            .nodes
            .borrow_mut()
            .get_mut(&visual.id())
            .unwrap()
            .rect = RECT {
            left: 10,
            top: 20,
            right: 30,
            bottom: 40,
        };

        assert!(visual.contains_client_point(windows::Win32::Foundation::POINT { x: 10, y: 20 }));
        assert!(visual.contains_client_point(windows::Win32::Foundation::POINT { x: 29, y: 39 }));
        assert!(!visual.contains_client_point(windows::Win32::Foundation::POINT { x: 30, y: 40 }));
    }

    #[test]
    fn flattened_native_order_follows_visual_depth_first_order() {
        let tree = Rc::new(TreeContext::new());
        let surface = VisualSurface::new(HWND::default(), tree.clone(), None);
        let first_hwnd = HWND(1usize as *mut c_void);
        let second_hwnd = HWND(2usize as *mut c_void);
        let root = surface.mount(None, tree.create_node(false), None);
        let first = surface.mount(Some(root.id()), tree.create_node(false), Some(first_hwnd));
        surface.mount(Some(first.id()), tree.create_node(true), Some(second_hwnd));

        let mut native = Vec::new();
        surface.collect_native(root.id(), &mut native);

        assert_eq!(native, vec![first_hwnd, second_hwnd]);
    }

    #[test]
    fn dirty_rect_union_covers_old_and_new_bounds() {
        let first = RECT {
            left: 20,
            top: 30,
            right: 80,
            bottom: 50,
        };
        let second = RECT {
            left: 15,
            top: 35,
            right: 90,
            bottom: 60,
        };

        assert_eq!(
            union_rects(first, second),
            RECT {
                left: 15,
                top: 30,
                right: 90,
                bottom: 60,
            }
        );
    }

    #[test]
    fn native_control_inherits_virtual_background_color() {
        let tree = Rc::new(TreeContext::new());
        let surface = VisualSurface::new(HWND::default(), tree.clone(), None);
        let parent = surface.mount(None, tree.create_node(false), None);
        let hwnd = HWND(1usize as *mut c_void);
        surface.mount(Some(parent.id()), tree.create_node(true), Some(hwnd));
        surface
            .nodes
            .borrow_mut()
            .get_mut(&parent.id())
            .unwrap()
            .background_color = Some(Color::RGB(nestix_native_core::RGBColor::from_rgb(
            0x12, 0x34, 0x56,
        )));

        assert_eq!(surface.control_background_color(hwnd), COLORREF(0x56_34_12));
    }
}
