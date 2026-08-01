use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{
        Once,
        atomic::{AtomicUsize, Ordering},
    },
};

use nestix::{
    Element, Layout, Shared, StateSetter, callback, closure, component,
    components::ContextProvider, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    CollectionNodeProps, ListViewHostProps, StyleContext, StyleScope, TableViewHostProps,
    TreeViewHostProps, dpi::LogicalSize, matched_style, resolved_view_style,
};
pub use nestix_native_core::{
    ListView, ListViewItem, ListViewItemProps, ListViewProps, TableView, TableViewCell,
    TableViewCellProps, TableViewColumn, TableViewProps, TableViewRow, TableViewRowProps, TreeView,
    TreeViewItem, TreeViewItemProps, TreeViewProps,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        Graphics::Gdi::{DeleteObject, HFONT},
        UI::{
            Controls::{
                HTREEITEM, ICC_LISTVIEW_CLASSES, ICC_TREEVIEW_CLASSES, INITCOMMONCONTROLSEX,
                InitCommonControlsEx, LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_STATE,
                LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS,
                LVM_DELETECOLUMN, LVM_ENSUREVISIBLE, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW,
                LVM_INSERTITEMW, LVM_SETCOLUMNWIDTH, LVM_SETEXTENDEDLISTVIEWSTYLE,
                LVM_SETITEMSTATE, LVM_SETITEMW, LVN_ITEMCHANGED, LVNI_SELECTED,
                LVS_EX_FULLROWSELECT, LVS_NOCOLUMNHEADER, LVS_REPORT, LVS_SHOWSELALWAYS,
                LVS_SINGLESEL, NM_DBLCLK, NM_RETURN, NMHDR, NMITEMACTIVATE, NMLISTVIEW,
                NMTREEVIEWW, TVE_COLLAPSE, TVE_EXPAND, TVGN_CARET, TVI_LAST, TVI_ROOT,
                TVIF_CHILDREN, TVIF_PARAM, TVIF_TEXT, TVINSERTSTRUCTW, TVITEMEXW_CHILDREN, TVITEMW,
                TVM_DELETEITEM, TVM_ENSUREVISIBLE as TVM_ENSUREVISIBLE_ITEM, TVM_EXPAND,
                TVM_GETNEXTITEM, TVM_INSERTITEMW, TVM_SELECTITEM, TVN_ITEMEXPANDEDW,
                TVN_SELCHANGEDW, TVS_DISABLEDRAGDROP, TVS_FULLROWSELECT, TVS_HASBUTTONS,
                TVS_HASLINES, TVS_LINESATROOT, TVS_SHOWSELALWAYS, WC_LISTVIEWW, WC_TREEVIEWW,
            },
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, GetClientRect, SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_NOTIFY, WM_SETFONT, WS_BORDER, WS_CHILD, WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
    core::PWSTR,
};

use crate::{AppState, WindowContext, contexts::ParentContext, font::ui_font, native_control};

static NEXT_REGISTRATION_ID: AtomicUsize = AtomicUsize::new(1);

fn init_common_controls() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES | ICC_TREEVIEW_CLASSES,
        };
        let _ = InitCommonControlsEx(&controls);
    });
}

#[derive(Default)]
struct RowDefinition {
    registration_id: usize,
    value: RefCell<String>,
    label: RefCell<String>,
    enabled: Cell<bool>,
    cells: RefCell<HashMap<String, String>>,
}

#[derive(Clone)]
struct FlatContext {
    rows: Rc<RefCell<Vec<Rc<RowDefinition>>>>,
    set_revision: StateSetter<usize>,
}

impl FlatContext {
    fn changed(&self) {
        self.set_revision
            .mutate(|value| *value = value.wrapping_add(1));
    }
}

#[derive(Clone)]
struct RowContext {
    row: Rc<RowDefinition>,
    changed: Shared<dyn Fn()>,
}

#[derive(Clone, Copy)]
enum FlatKind {
    List,
    Table,
}

struct FlatState {
    rows: Rc<RefCell<Vec<Rc<RowDefinition>>>>,
    suppress_selection_change: Cell<bool>,
    column_count: Cell<usize>,
    needs_column_layout: Cell<bool>,
}

#[allow(clippy::too_many_arguments)]
fn mount_flat(
    element: &Element,
    class: nestix::PropValue<nestix_native_core::ClassList>,
    view: &nestix_native_core::ViewProps,
    enabled: nestix::PropValue<bool>,
    value: nestix::PropValue<Option<String>>,
    on_value_change: nestix::PropValue<Option<Shared<dyn Fn(&str)>>>,
    on_activate: nestix::PropValue<Option<Shared<dyn Fn(&str)>>>,
    children: Layout,
    kind: FlatKind,
    columns: nestix::PropValue<Vec<TableViewColumn>>,
    default_classes: &'static [&'static str],
) -> Element {
    let app_state = element.context::<AppState>().unwrap();
    let window = element.context::<WindowContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        class.clone(),
        default_classes,
    );
    let effective_style = resolved_view_style(matched, view);
    let rows = Rc::new(RefCell::new(Vec::new()));
    let (revision, set_revision) = create_state(0usize);
    let state = Rc::new(FlatState {
        rows: rows.clone(),
        suppress_selection_change: Cell::new(false),
        column_count: Cell::new(0),
        needs_column_layout: Cell::new(true),
    });

    init_common_controls();
    let mut native_style = WS_CHILD
        | WS_VISIBLE
        | WS_TABSTOP
        | WS_BORDER
        | WINDOW_STYLE(LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS);
    if matches!(kind, FlatKind::List) {
        native_style |= WINDOW_STYLE(LVS_NOCOLUMNHEADER);
    }
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_LISTVIEWW,
            None,
            native_style,
            0,
            0,
            0,
            0,
            Some(parent.surface.hwnd()),
            None,
            None,
            None,
        )
        .unwrap()
    };
    unsafe {
        SendMessageW(
            hwnd,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            Some(WPARAM(LVS_EX_FULLROWSELECT as usize)),
            Some(LPARAM(LVS_EX_FULLROWSELECT as isize)),
        );
    }
    let intrinsic = match kind {
        FlatKind::List => LogicalSize::new(240.0, 160.0),
        FlatKind::Table => LogicalSize::new(320.0, 180.0),
    };
    let (intrinsic, _) = create_state(intrinsic);
    native_control::mount(
        element,
        hwnd,
        effective_style.clone(),
        view,
        intrinsic.into_readonly(),
    );
    let weak_state = Rc::downgrade(&state);
    let after_sync_id = parent.surface.add_after_sync(move |_| {
        if let Some(state) = weak_state.upgrade() {
            resize_flat_columns(hwnd, &state, kind);
        }
    });

    app_state.add_control_handler(hwnd, callback!(
        [state, enabled, value, on_value_change, on_activate]
        |msg: u32, _: WPARAM, lparam: LPARAM| {
            if msg != WM_NOTIFY { return; }
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            match header.code {
                LVN_ITEMCHANGED => {
                    if state.suppress_selection_change.get() { return; }
                    let notification = unsafe { &*(lparam.0 as *const NMLISTVIEW) };
                    if notification.iItem < 0
                        || notification.uOldState & LVIS_SELECTED.0 != 0
                        || notification.uNewState & LVIS_SELECTED.0 == 0
                    { return; }
                    let row = state.rows.borrow().get(notification.iItem as usize).cloned();
                    if let Some(row) = row {
                        if enabled.get() && row.enabled.get() {
                            if let Some(callback) = on_value_change.get() {
                                callback(&row.value.borrow());
                            }
                        } else {
                            apply_flat_selection(hwnd, &state, value.get().as_deref(), enabled.get());
                        }
                    }
                }
                NM_DBLCLK => {
                    let activation = unsafe { &*(lparam.0 as *const NMITEMACTIVATE) };
                    invoke_flat_activation(&state, activation.iItem, enabled.get(), &on_activate);
                }
                NM_RETURN => {
                    let index = unsafe {
                        SendMessageW(hwnd, LVM_GETNEXTITEM, Some(WPARAM(usize::MAX)), Some(LPARAM(LVNI_SELECTED as isize))).0 as i32
                    };
                    invoke_flat_activation(&state, index, enabled.get(), &on_activate);
                }
                _ => {}
            }
        }
    ));

    let font = Rc::new(Cell::new(None::<HFONT>));
    element.on_unmount(closure!(
        [app_state, parent, font] || {
            parent.surface.remove_after_sync(after_sync_id);
            app_state.remove_control_handler(hwnd);
            if let Some(font) = font.take() {
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    ));
    scoped_effect!(
        [window.scale_factor, font]
            || unsafe {
                let next = ui_font(12.0, scale_factor.get());
                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(next.0 as usize)),
                    Some(LPARAM(1)),
                );
                if let Some(previous) = font.replace(Some(next)) {
                    let _ = DeleteObject(previous.into());
                }
            }
    );
    scoped_effect!(
        [enabled]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
            }
    );
    scoped_effect!(
        [state, revision, columns, value, enabled] || {
            let _ = revision.get();
            rebuild_flat(hwnd, &state, kind, unique_columns(columns.get()));
            apply_flat_selection(hwnd, &state, value.get().as_deref(), enabled.get());
        }
    );

    layout! {
        StyleScope(
            .class = class,
            .default_classes = default_classes,
            .effective_style = effective_style,
        ) {
            ContextProvider<FlatContext>(FlatContext { rows, set_revision }) {
                $(children)
            }
        }
    }
}

fn invoke_flat_activation(
    state: &FlatState,
    index: i32,
    enabled: bool,
    callback: &nestix::PropValue<Option<Shared<dyn Fn(&str)>>>,
) {
    if !enabled || index < 0 {
        return;
    }
    let row = state.rows.borrow().get(index as usize).cloned();
    if let Some(row) = row.filter(|row| row.enabled.get())
        && let Some(callback) = callback.get()
    {
        callback(&row.value.borrow());
    }
}

fn rebuild_flat(hwnd: HWND, state: &FlatState, kind: FlatKind, columns: Vec<TableViewColumn>) {
    state.suppress_selection_change.set(true);
    unsafe {
        SendMessageW(hwnd, LVM_DELETEALLITEMS, None, None);
        while SendMessageW(hwnd, LVM_DELETECOLUMN, Some(WPARAM(0)), None).0 != 0 {}
    }
    let definitions = match kind {
        FlatKind::List => vec![TableViewColumn::new("__list", "")],
        FlatKind::Table => columns,
    };
    state.column_count.set(definitions.len());
    state.needs_column_layout.set(true);
    for (index, definition) in definitions.iter().enumerate() {
        let mut text = wide(&definition.title);
        let mut column = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
            cx: if definitions.is_empty() { 160 } else { 180 },
            pszText: PWSTR(text.as_mut_ptr()),
            iSubItem: index as i32,
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                hwnd,
                LVM_INSERTCOLUMNW,
                Some(WPARAM(index)),
                Some(LPARAM(&mut column as *mut _ as isize)),
            );
        }
    }
    let rows = state.rows.borrow().clone();
    for (index, row) in rows.iter().enumerate() {
        let first = match kind {
            FlatKind::List => row.label.borrow().clone(),
            FlatKind::Table => definitions
                .first()
                .and_then(|column| row.cells.borrow().get(&column.id).cloned())
                .unwrap_or_default(),
        };
        set_list_item(hwnd, index, 0, &first, true);
        if matches!(kind, FlatKind::Table) {
            for (subitem, column) in definitions.iter().enumerate().skip(1) {
                let text = row
                    .cells
                    .borrow()
                    .get(&column.id)
                    .cloned()
                    .unwrap_or_default();
                set_list_item(hwnd, index, subitem, &text, false);
            }
        }
    }
    resize_flat_columns(hwnd, state, kind);
    state.suppress_selection_change.set(false);
}

fn resize_flat_columns(hwnd: HWND, state: &FlatState, kind: FlatKind) {
    if matches!(kind, FlatKind::Table) && !state.needs_column_layout.replace(false) {
        return;
    }
    let count = state.column_count.get();
    if count == 0 {
        return;
    }
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
    }
    let width = rect.right - rect.left;
    if width <= 0 {
        state.needs_column_layout.set(true);
        return;
    }
    let base_width = (width / count as i32).max(1);
    for column in 0..count {
        let column_width = if column + 1 == count {
            width - base_width * column as i32
        } else {
            base_width
        };
        unsafe {
            SendMessageW(
                hwnd,
                LVM_SETCOLUMNWIDTH,
                Some(WPARAM(column)),
                Some(LPARAM(column_width as isize)),
            );
        }
    }
}

fn set_list_item(hwnd: HWND, row: usize, column: usize, text: &str, insert: bool) {
    let mut text = wide(text);
    let mut item = LVITEMW {
        mask: LVIF_TEXT,
        iItem: row as i32,
        iSubItem: column as i32,
        pszText: PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            hwnd,
            if insert {
                LVM_INSERTITEMW
            } else {
                LVM_SETITEMW
            },
            None,
            Some(LPARAM(&mut item as *mut _ as isize)),
        );
    }
}

fn apply_flat_selection(hwnd: HWND, state: &FlatState, value: Option<&str>, enabled: bool) {
    let index = enabled
        .then(|| {
            value.and_then(|value| {
                state
                    .rows
                    .borrow()
                    .iter()
                    .position(|row| row.enabled.get() && row.value.borrow().as_str() == value)
            })
        })
        .flatten();
    state.suppress_selection_change.set(true);
    let selection_state =
        windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0);
    let mut clear = LVITEMW {
        mask: LVIF_STATE,
        stateMask: selection_state,
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            hwnd,
            LVM_SETITEMSTATE,
            Some(WPARAM(usize::MAX)),
            Some(LPARAM(&mut clear as *mut _ as isize)),
        );
    }
    if let Some(index) = index {
        let mut select = LVITEMW {
            mask: LVIF_STATE,
            state: selection_state,
            stateMask: selection_state,
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                hwnd,
                LVM_SETITEMSTATE,
                Some(WPARAM(index)),
                Some(LPARAM(&mut select as *mut _ as isize)),
            );
            SendMessageW(
                hwnd,
                LVM_ENSUREVISIBLE,
                Some(WPARAM(index)),
                Some(LPARAM(0)),
            );
        }
    }
    state.suppress_selection_change.set(false);
}

fn unique_columns(columns: Vec<TableViewColumn>) -> Vec<TableViewColumn> {
    let mut ids = HashSet::new();
    columns
        .into_iter()
        .filter(|column| ids.insert(column.id.clone()))
        .collect()
}

#[component]
pub(crate) fn ListViewHost(props: &ListViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, ListView, output);
    const CLASSES: [&str; 2] = ["__ListView", "__win32_ListView"];
    mount_flat(
        element,
        props.class.clone(),
        &props.view,
        props.enabled.clone(),
        props.value.clone(),
        props.on_value_change.clone(),
        props.on_activate.clone(),
        props.children.get(),
        FlatKind::List,
        nestix::PropValue::from_plain(Vec::new()),
        &CLASSES,
    )
}

#[component]
pub(crate) fn TableViewHost(props: &TableViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, TableView, output);
    const CLASSES: [&str; 2] = ["__TableView", "__win32_TableView"];
    mount_flat(
        element,
        props.class.clone(),
        &props.view,
        props.enabled.clone(),
        props.value.clone(),
        props.on_value_change.clone(),
        props.on_activate.clone(),
        props.children.get(),
        FlatKind::Table,
        props.columns.clone(),
        &CLASSES,
    )
}

#[component]
pub(crate) fn ListViewNodeHost(props: &CollectionNodeProps, element: &Element) -> Element {
    register_flat_node(props, element)
}

#[component]
pub(crate) fn TableViewNodeHost(props: &CollectionNodeProps, element: &Element) -> Element {
    register_flat_node(props, element)
}

fn register_flat_node(props: &CollectionNodeProps, element: &Element) -> Element {
    let context = element
        .context::<FlatContext>()
        .expect("row must be mounted beneath its view");
    let row = Rc::new(RowDefinition {
        registration_id: NEXT_REGISTRATION_ID.fetch_add(1, Ordering::Relaxed),
        value: RefCell::new(props.value.get()),
        enabled: Cell::new(true),
        ..Default::default()
    });
    element.on_place(closure!(
        [context, row] | placement | {
            let mut rows = context.rows.borrow_mut();
            rows.retain(|candidate| candidate.registration_id != row.registration_id);
            let index = placement.index.unwrap_or(rows.len()).min(rows.len());
            rows.insert(index, row.clone());
            drop(rows);
            context.changed();
        }
    ));
    element.on_unmount(closure!(
        [context, row] || {
            context
                .rows
                .borrow_mut()
                .retain(|candidate| candidate.registration_id != row.registration_id);
            context.changed();
        }
    ));
    scoped_effect!(
        [context, row, props.value] || {
            *row.value.borrow_mut() = value.get();
            context.changed();
        }
    );
    let changed: Shared<dyn Fn()> = Shared::from(Rc::new({
        let context = context.clone();
        move || context.changed()
    }) as Rc<dyn Fn()>);
    layout! {
        ContextProvider<RowContext>(RowContext { row, changed }) {
            $(props.children.clone())
        }
    }
}

#[component]
pub(crate) fn ListViewItemHost(props: &ListViewItemProps, element: &Element) {
    require_visual_mount!(element, ListViewItem);
    let context = element
        .context::<RowContext>()
        .expect("ListViewItem must be rendered by ListView");
    scoped_effect!(
        [context, props.label, props.enabled] || {
            *context.row.label.borrow_mut() = label.get();
            context.row.enabled.set(enabled.get());
            (context.changed)();
        }
    );
}

#[component]
pub(crate) fn TableViewRowHost(props: &TableViewRowProps, element: &Element) -> Element {
    require_visual_mount!(element, TableViewRow, output);
    let context = element
        .context::<RowContext>()
        .expect("TableViewRow must be rendered by TableView");
    scoped_effect!(
        [context, props.enabled] || {
            context.row.enabled.set(enabled.get());
            (context.changed)();
        }
    );
    layout! { nestix::components::Fragment(.children = props.children.clone()) }
}

#[component]
pub(crate) fn TableViewCellHost(props: &TableViewCellProps, element: &Element) {
    require_visual_mount!(element, TableViewCell);
    let context = element
        .context::<RowContext>()
        .expect("TableViewCell must be beneath TableViewRow");
    let previous = Rc::new(RefCell::new(None::<String>));
    element.on_unmount(closure!(
        [context, previous] || {
            if let Some(column) = previous.borrow_mut().take() {
                context.row.cells.borrow_mut().remove(&column);
                (context.changed)();
            }
        }
    ));
    scoped_effect!(
        [context, previous, props.column, props.text] || {
            let column = column.get();
            if let Some(old) = previous.borrow_mut().replace(column.clone())
                && old != column
            {
                context.row.cells.borrow_mut().remove(&old);
            }
            context.row.cells.borrow_mut().insert(column, text.get());
            (context.changed)();
        }
    );
}

#[derive(Default)]
struct TreeNodeDefinition {
    registration_id: usize,
    value: RefCell<String>,
    label: RefCell<String>,
    enabled: Cell<bool>,
    children: Rc<RefCell<Vec<Rc<TreeNodeDefinition>>>>,
}

#[derive(Clone)]
struct TreeParentContext {
    children: Rc<RefCell<Vec<Rc<TreeNodeDefinition>>>>,
    changed: Shared<dyn Fn()>,
}

#[derive(Clone)]
struct TreeNodeContext {
    node: Rc<TreeNodeDefinition>,
    changed: Shared<dyn Fn()>,
}

struct TreeState {
    roots: Rc<RefCell<Vec<Rc<TreeNodeDefinition>>>>,
    handles: RefCell<HashMap<isize, Rc<TreeNodeDefinition>>>,
    expanded: RefCell<HashSet<String>>,
    suppress_selection_change: Cell<bool>,
}

#[component]
pub(crate) fn TreeViewHost(props: &TreeViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, TreeView, output);
    const CLASSES: [&str; 2] = ["__TreeView", "__win32_TreeView"];
    let app_state = element.context::<AppState>().unwrap();
    let window = element.context::<WindowContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    let roots = Rc::new(RefCell::new(Vec::new()));
    let (revision, set_revision) = create_state(0usize);
    let state = Rc::new(TreeState {
        roots: roots.clone(),
        handles: RefCell::new(HashMap::new()),
        expanded: RefCell::new(HashSet::new()),
        suppress_selection_change: Cell::new(false),
    });
    init_common_controls();
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_TREEVIEWW,
            None,
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_BORDER
                | WINDOW_STYLE(
                    TVS_HASBUTTONS
                        | TVS_HASLINES
                        | TVS_LINESATROOT
                        | TVS_SHOWSELALWAYS
                        | TVS_DISABLEDRAGDROP
                        | TVS_FULLROWSELECT,
                ),
            0,
            0,
            0,
            0,
            Some(parent.surface.hwnd()),
            None,
            None,
            None,
        )
        .unwrap()
    };
    let (intrinsic, _) = create_state(LogicalSize::new(240.0, 180.0));
    native_control::mount(
        element,
        hwnd,
        effective_style.clone(),
        &props.view,
        intrinsic.into_readonly(),
    );
    app_state.add_control_handler(hwnd, callback!(
        [state, props.enabled, props.value, props.on_value_change, props.on_activate]
        |msg: u32, _: WPARAM, lparam: LPARAM| {
            if msg != WM_NOTIFY { return; }
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            match header.code {
                TVN_SELCHANGEDW => {
                    if state.suppress_selection_change.get() { return; }
                    let notification = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                    let node = state.handles.borrow().get(&notification.itemNew.hItem.0).cloned();
                    if let Some(node) = node {
                        if enabled.get() && node.enabled.get() {
                            if let Some(callback) = on_value_change.get() { callback(&node.value.borrow()); }
                        } else {
                            apply_tree_selection(hwnd, &state, value.get().as_deref(), enabled.get());
                        }
                    }
                }
                TVN_ITEMEXPANDEDW => {
                    if state.suppress_selection_change.get() {
                        return;
                    }
                    let notification = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                    if let Some(node) = state.handles.borrow().get(&notification.itemNew.hItem.0) {
                        let key = node.value.borrow().clone();
                        if notification.action == TVE_EXPAND { state.expanded.borrow_mut().insert(key); }
                        else if notification.action == TVE_COLLAPSE { state.expanded.borrow_mut().remove(&key); }
                    }
                }
                NM_DBLCLK | NM_RETURN => invoke_tree_activation(hwnd, &state, enabled.get(), &on_activate),
                _ => {}
            }
        }
    ));
    let font = Rc::new(Cell::new(None::<HFONT>));
    element.on_unmount(closure!(
        [app_state, font] || {
            app_state.remove_control_handler(hwnd);
            if let Some(font) = font.take() {
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    ));
    scoped_effect!(
        [window.scale_factor, font]
            || unsafe {
                let next = ui_font(12.0, scale_factor.get());
                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(next.0 as usize)),
                    Some(LPARAM(1)),
                );
                if let Some(previous) = font.replace(Some(next)) {
                    let _ = DeleteObject(previous.into());
                }
            }
    );
    scoped_effect!(
        [props.enabled]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
            }
    );
    scoped_effect!(
        [state, revision, props.value, props.enabled] || {
            let _ = revision.get();
            rebuild_tree(hwnd, &state);
            apply_tree_selection(hwnd, &state, value.get().as_deref(), enabled.get());
        }
    );
    let changed: Shared<dyn Fn()> = Shared::from(Rc::new(move || {
        set_revision.mutate(|revision| *revision = revision.wrapping_add(1));
    }) as Rc<dyn Fn()>);
    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<TreeParentContext>(TreeParentContext { children: roots, changed }) {
                $(props.children.clone())
            }
        }
    }
}

fn rebuild_tree(hwnd: HWND, state: &TreeState) {
    state.suppress_selection_change.set(true);
    unsafe {
        SendMessageW(hwnd, TVM_DELETEITEM, None, Some(LPARAM(TVI_ROOT.0)));
    }
    state.handles.borrow_mut().clear();
    let roots = state.roots.borrow().clone();
    for root in roots {
        insert_tree_node(hwnd, state, TVI_ROOT, &root);
    }
    state.suppress_selection_change.set(false);
}

fn insert_tree_node(
    hwnd: HWND,
    state: &TreeState,
    parent: HTREEITEM,
    node: &Rc<TreeNodeDefinition>,
) {
    let mut text = wide(&node.label.borrow());
    let mut insert = TVINSERTSTRUCTW {
        hParent: parent,
        hInsertAfter: TVI_LAST,
        Anonymous: windows::Win32::UI::Controls::TVINSERTSTRUCTW_0 {
            item: TVITEMW {
                mask: TVIF_TEXT | TVIF_PARAM | TVIF_CHILDREN,
                pszText: PWSTR(text.as_mut_ptr()),
                cChildren: TVITEMEXW_CHILDREN((!node.children.borrow().is_empty()) as i32),
                ..Default::default()
            },
        },
    };
    let handle = HTREEITEM(unsafe {
        SendMessageW(
            hwnd,
            TVM_INSERTITEMW,
            None,
            Some(LPARAM(&mut insert as *mut _ as isize)),
        )
        .0
    });
    state.handles.borrow_mut().insert(handle.0, node.clone());
    for child in node.children.borrow().clone() {
        insert_tree_node(hwnd, state, handle, &child);
    }
    if state
        .expanded
        .borrow()
        .contains(node.value.borrow().as_str())
    {
        unsafe {
            SendMessageW(
                hwnd,
                TVM_EXPAND,
                Some(WPARAM(TVE_EXPAND.0 as usize)),
                Some(LPARAM(handle.0)),
            );
        }
    }
}

fn apply_tree_selection(hwnd: HWND, state: &TreeState, value: Option<&str>, enabled: bool) {
    let handle = enabled
        .then(|| {
            value.and_then(|value| {
                state.handles.borrow().iter().find_map(|(handle, node)| {
                    (node.enabled.get() && node.value.borrow().as_str() == value).then_some(*handle)
                })
            })
        })
        .flatten();
    state.suppress_selection_change.set(true);
    unsafe {
        SendMessageW(
            hwnd,
            TVM_SELECTITEM,
            Some(WPARAM(TVGN_CARET as usize)),
            Some(LPARAM(handle.unwrap_or_default())),
        );
        if let Some(handle) = handle {
            SendMessageW(hwnd, TVM_ENSUREVISIBLE_ITEM, None, Some(LPARAM(handle)));
        }
    }
    state.suppress_selection_change.set(false);
}

fn invoke_tree_activation(
    hwnd: HWND,
    state: &TreeState,
    enabled: bool,
    callback: &nestix::PropValue<Option<Shared<dyn Fn(&str)>>>,
) {
    if !enabled {
        return;
    }
    let handle = unsafe {
        SendMessageW(
            hwnd,
            TVM_GETNEXTITEM,
            Some(WPARAM(TVGN_CARET as usize)),
            None,
        )
        .0
    };
    let node = state.handles.borrow().get(&handle).cloned();
    if let Some(node) = node.filter(|node| node.enabled.get())
        && let Some(callback) = callback.get()
    {
        callback(&node.value.borrow());
    }
}

#[component]
pub(crate) fn TreeViewNodeHost(props: &CollectionNodeProps, element: &Element) -> Element {
    let parent = element
        .context::<TreeParentContext>()
        .expect("tree node must be beneath TreeView");
    let node = Rc::new(TreeNodeDefinition {
        registration_id: NEXT_REGISTRATION_ID.fetch_add(1, Ordering::Relaxed),
        value: RefCell::new(props.value.get()),
        children: Rc::new(RefCell::new(Vec::new())),
        ..Default::default()
    });
    element.on_place(closure!(
        [parent, node] | placement | {
            let mut children = parent.children.borrow_mut();
            children.retain(|candidate| candidate.registration_id != node.registration_id);
            let index = placement
                .index
                .unwrap_or(children.len())
                .min(children.len());
            children.insert(index, node.clone());
            drop(children);
            (parent.changed)();
        }
    ));
    element.on_unmount(closure!(
        [parent, node] || {
            parent
                .children
                .borrow_mut()
                .retain(|candidate| candidate.registration_id != node.registration_id);
            (parent.changed)();
        }
    ));
    scoped_effect!(
        [parent, node, props.value] || {
            *node.value.borrow_mut() = value.get();
            (parent.changed)();
        }
    );
    layout! {
        ContextProvider<TreeNodeContext>(
            TreeNodeContext { node: node.clone(), changed: parent.changed.clone() },
        ) {
            ContextProvider<TreeParentContext>(
                TreeParentContext {
                    children: node.children.clone(),
                    changed: parent.changed.clone(),
                },
            ) {
                $(props.children.clone())
            }
        }
    }
}

#[component]
pub(crate) fn TreeViewItemHost(props: &TreeViewItemProps, element: &Element) {
    require_visual_mount!(element, TreeViewItem);
    let context = element
        .context::<TreeNodeContext>()
        .expect("TreeViewItem must be rendered by TreeView");
    scoped_effect!(
        [context, props.label, props.enabled] || {
            *context.node.label.borrow_mut() = label.get();
            context.node.enabled.set(enabled.get());
            (context.changed)();
        }
    );
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_columns_keep_the_first_definition() {
        assert_eq!(
            unique_columns(vec![
                TableViewColumn::new("name", "Name"),
                TableViewColumn::new("name", "Replacement"),
                TableViewColumn::new("role", "Role"),
            ]),
            vec![
                TableViewColumn::new("name", "Name"),
                TableViewColumn::new("role", "Role")
            ]
        );
    }
}
