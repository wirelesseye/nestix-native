use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

use nestix::{
    Element, PropValue, Shared, StateSetter, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::dpi::LogicalSize;
use nestix_native_core::{
    CollectionNodeProps, ListViewHostProps, StyleContext, StyleScope, TableViewHostProps,
    TreeViewHostProps, matched_style, resolved_view_style,
};
pub use nestix_native_core::{
    ListView, ListViewItem, ListViewItemProps, ListViewProps, TableView, TableViewCell,
    TableViewCellProps, TableViewColumn, TableViewProps, TableViewRow, TableViewRowProps, TreeView,
    TreeViewItem, TreeViewItemProps, TreeViewProps,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSControlTextEditingDelegate, NSEvent, NSOutlineView, NSOutlineViewDataSource,
    NSOutlineViewDelegate, NSScrollView, NSTableColumn, NSTableColumnResizingOptions, NSTableView,
    NSTableViewDataSource, NSTableViewDelegate, NSTableViewStyle, NSTextField, NSView,
};
use objc2_foundation::{NSIndexSet, NSNotification, NSObject, NSObjectProtocol, NSString};

use crate::native_control;

thread_local! {
    static FLAT_HANDLERS: RefCell<HashMap<String, Retained<FlatHandler>>> = RefCell::new(HashMap::new());
    static TREE_HANDLERS: RefCell<HashMap<String, Retained<TreeHandler>>> = RefCell::new(HashMap::new());
}

#[derive(Default)]
struct RowDefinition {
    registration_id: String,
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
            .mutate(|revision| *revision = revision.wrapping_add(1));
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

struct FlatHandlerState {
    kind: FlatKind,
    rows: Rc<RefCell<Vec<Rc<RowDefinition>>>>,
    on_value_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
    on_activate: PropValue<Option<Shared<dyn Fn(&str)>>>,
    enabled: PropValue<bool>,
    suppress_selection_change: Cell<bool>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FlatHandlerState]
    struct FlatHandler;

    unsafe impl NSObjectProtocol for FlatHandler {}
    unsafe impl NSControlTextEditingDelegate for FlatHandler {}

    unsafe impl NSTableViewDataSource for FlatHandler {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _: &NSTableView) -> isize {
            self.ivars().rows.borrow().len() as isize
        }
    }

    unsafe impl NSTableViewDelegate for FlatHandler {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(
            &self,
            _: &NSTableView,
            column: Option<&NSTableColumn>,
            row: isize,
        ) -> Option<Retained<NSView>> {
            let rows = self.ivars().rows.borrow();
            rows.get(row as usize).and_then(|row| {
                let text = match self.ivars().kind {
                    FlatKind::List => row.label.borrow().clone(),
                    FlatKind::Table => {
                        let id = column.map(|column| column.identifier().to_string())?;
                        row.cells.borrow().get(&id).cloned().unwrap_or_default()
                    }
                };
                let label = NSTextField::labelWithString(&NSString::from_str(&text), self.mtm());
                label.setEnabled(self.ivars().enabled.get() && row.enabled.get());
                Some(label.into_super().into_super())
            })
        }

        #[unsafe(method(tableView:shouldSelectRow:))]
        fn should_select_row(&self, _: &NSTableView, row: isize) -> bool {
            self.ivars().enabled.get()
                && self
                    .ivars()
                    .rows
                    .borrow()
                    .get(row as usize)
                    .is_some_and(|row| row.enabled.get())
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_did_change(&self, notification: &NSNotification) {
            if self.ivars().suppress_selection_change.get() {
                return;
            }
            let Some(table) = notification
                .object()
                .and_then(|object| object.downcast::<NSTableView>().ok())
            else {
                return;
            };
            if let Some(value) = self.value_at_row(table.selectedRow())
                && let Some(callback) = self.ivars().on_value_change.get()
            {
                callback(&value);
            }
        }
    }

    impl FlatHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, sender: &NSTableView) {
            self.invoke_activation(sender);
        }
    }
);

impl FlatHandler {
    fn new(mtm: MainThreadMarker, state: FlatHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }

    fn value_at_row(&self, row: isize) -> Option<String> {
        if !self.ivars().enabled.get() || row < 0 {
            return None;
        }
        self.ivars()
            .rows
            .borrow()
            .get(row as usize)
            .filter(|row| row.enabled.get())
            .map(|row| row.value.borrow().clone())
    }

    fn invoke_activation(&self, table: &NSTableView) {
        let row = if table.clickedRow() >= 0 {
            table.clickedRow()
        } else {
            table.selectedRow()
        };
        if let Some(value) = self.value_at_row(row)
            && let Some(callback) = self.ivars().on_activate.get()
        {
            callback(&value);
        }
    }

    fn apply_selection(&self, table: &NSTableView, value: Option<&str>) {
        if !self.ivars().enabled.get() {
            self.ivars().suppress_selection_change.set(true);
            unsafe { table.deselectAll(None) };
            self.ivars().suppress_selection_change.set(false);
            return;
        }
        let index = value.and_then(|value| {
            self.ivars()
                .rows
                .borrow()
                .iter()
                .position(|row| row.enabled.get() && row.value.borrow().as_str() == value)
        });
        self.ivars().suppress_selection_change.set(true);
        if let Some(index) = index {
            table.selectRowIndexes_byExtendingSelection(
                &NSIndexSet::indexSetWithIndex(index),
                false,
            );
            table.scrollRowToVisible(index as isize);
        } else {
            unsafe { table.deselectAll(None) };
        }
        self.ivars().suppress_selection_change.set(false);
    }
}

struct ActivatingTableViewState {
    handler: Retained<FlatHandler>,
}

define_class!(
    #[unsafe(super = NSTableView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActivatingTableViewState]
    struct ActivatingTableView;

    unsafe impl NSObjectProtocol for ActivatingTableView {}

    impl ActivatingTableView {
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if matches!(event.keyCode(), 36 | 76) {
                self.ivars().handler.invoke_activation(self);
            } else {
                unsafe { msg_send![super(self), keyDown: event] }
            }
        }
    }
);

impl ActivatingTableView {
    fn new(mtm: MainThreadMarker, handler: Retained<FlatHandler>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ActivatingTableViewState { handler });
        unsafe { msg_send![super(this), init] }
    }
}

fn configure_table(table: &NSTableView) {
    table.setAllowsMultipleSelection(false);
    table.setAllowsEmptySelection(true);
    table.setAllowsColumnReordering(false);
    table.setStyle(NSTableViewStyle::FullWidth);
}

fn mount_flat(
    element: &Element,
    class: PropValue<nestix_native_core::ClassList>,
    view: &nestix_native_core::ViewProps,
    enabled: PropValue<bool>,
    value: PropValue<Option<String>>,
    on_value_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
    on_activate: PropValue<Option<Shared<dyn Fn(&str)>>>,
    children: nestix::Layout,
    kind: FlatKind,
    columns: PropValue<Vec<TableViewColumn>>,
    default_classes: &'static [&'static str],
) -> Element {
    let matched_styles = matched_style(
        element.context::<StyleContext>(),
        element,
        class.clone(),
        default_classes,
    );
    let effective_style = resolved_view_style(matched_styles, view);
    let rows = Rc::new(RefCell::new(Vec::new()));
    let (revision, set_revision) = create_state(0usize);
    let mtm = MainThreadMarker::new().expect("collection views must mount on the main thread");
    let handler = FlatHandler::new(
        mtm,
        FlatHandlerState {
            kind,
            rows: rows.clone(),
            on_value_change,
            on_activate,
            enabled: enabled.clone(),
            suppress_selection_change: Cell::new(false),
        },
    );
    let table = ActivatingTableView::new(mtm, handler.clone());
    configure_table(&table);
    unsafe {
        table.setDataSource(Some(ProtocolObject::from_ref(&*handler)));
        table.setDelegate(Some(ProtocolObject::from_ref(&*handler)));
        table.setTarget(Some(&handler));
        table.setDoubleAction(Some(sel!(activate:)));
    }

    let scroll = NSScrollView::new(mtm);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(matches!(kind, FlatKind::Table));
    scroll.setDocumentView(Some(&table));
    native_control::mount_with_intrinsic_size(
        element,
        scroll.clone().into_super(),
        effective_style.clone(),
        view,
        revision.clone().into_readonly(),
        LogicalSize::new(240.0, 160.0),
    );

    let handler_id = nanoid::nanoid!();
    FLAT_HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler.clone()));
    element.on_unmount(closure!(
        [handler_id] || {
            FLAT_HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    scoped_effect!(
        [table, columns] || {
            let desired = unique_columns(columns.get());
            for column in table.tableColumns().iter() {
                table.removeTableColumn(&column);
            }
            if matches!(kind, FlatKind::List) {
                let column = NSTableColumn::initWithIdentifier(
                    NSTableColumn::alloc(mtm),
                    &NSString::from_str("__list"),
                );
                column.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
                table.addTableColumn(&column);
                table.setHeaderView(None);
            } else {
                for definition in desired {
                    let column = NSTableColumn::initWithIdentifier(
                        NSTableColumn::alloc(mtm),
                        &NSString::from_str(&definition.id),
                    );
                    column.setTitle(&NSString::from_str(&definition.title));
                    column.setResizingMask(
                        NSTableColumnResizingOptions::AutoresizingMask
                            | NSTableColumnResizingOptions::UserResizingMask,
                    );
                    table.addTableColumn(&column);
                }
            }
            table.reloadData();
        }
    );
    scoped_effect!(
        [table, handler, value, revision, enabled] || {
            let _ = revision.get();
            table.setEnabled(enabled.get());
            table.reloadData();
            handler.apply_selection(&table, value.get().as_deref());
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

fn unique_columns(columns: Vec<TableViewColumn>) -> Vec<TableViewColumn> {
    let mut ids = HashSet::new();
    columns
        .into_iter()
        .filter(|column| {
            if ids.insert(column.id.clone()) {
                true
            } else {
                log::warn!("duplicate TableView column id {:?} was ignored", column.id);
                false
            }
        })
        .collect()
}

#[component]
pub(crate) fn ListViewHost(props: &ListViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, ListView, output);
    const CLASSES: [&str; 2] = ["__ListView", "__appkit_ListView"];
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
        PropValue::from_plain(Vec::new()),
        &CLASSES,
    )
}

#[component]
pub(crate) fn TableViewHost(props: &TableViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, TableView, output);
    const CLASSES: [&str; 2] = ["__TableView", "__appkit_TableView"];
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
        registration_id: nanoid::nanoid!(),
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
    let previous_column = Rc::new(RefCell::new(None::<String>));
    element.on_unmount(closure!(
        [context, previous_column] || {
            if let Some(column) = previous_column.borrow_mut().take() {
                context.row.cells.borrow_mut().remove(&column);
                (context.changed)();
            }
        }
    ));
    scoped_effect!(
        [context, previous_column, props.column, props.text] || {
            let column = column.get();
            if let Some(previous) = previous_column.borrow_mut().replace(column.clone())
                && previous != column
            {
                context.row.cells.borrow_mut().remove(&previous);
            }
            context.row.cells.borrow_mut().insert(column, text.get());
            (context.changed)();
        }
    );
}

#[derive(Default)]
struct TreeNodeDefinition {
    registration_id: String,
    value: RefCell<String>,
    label: RefCell<String>,
    enabled: Cell<bool>,
    children: Rc<RefCell<Vec<Rc<TreeNodeDefinition>>>>,
    object: RefCell<Option<Retained<NSString>>>,
}

impl TreeNodeDefinition {
    fn object(&self) -> Retained<NSString> {
        if let Some(object) = self.object.borrow().as_ref() {
            return object.clone();
        }
        let object = NSString::from_str(&self.value.borrow());
        *self.object.borrow_mut() = Some(object.clone());
        object
    }
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

struct TreeHandlerState {
    roots: Rc<RefCell<Vec<Rc<TreeNodeDefinition>>>>,
    on_value_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
    on_activate: PropValue<Option<Shared<dyn Fn(&str)>>>,
    enabled: PropValue<bool>,
    suppress_selection_change: Cell<bool>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TreeHandlerState]
    struct TreeHandler;

    unsafe impl NSObjectProtocol for TreeHandler {}
    unsafe impl NSControlTextEditingDelegate for TreeHandler {}

    unsafe impl NSOutlineViewDataSource for TreeHandler {
        #[unsafe(method(outlineView:numberOfChildrenOfItem:))]
        unsafe fn number_of_children(&self, _: &NSOutlineView, item: Option<&AnyObject>) -> isize {
            self.children_for(item).len() as isize
        }

        #[unsafe(method_id(outlineView:child:ofItem:))]
        unsafe fn child(&self, _: &NSOutlineView, index: isize, item: Option<&AnyObject>) -> Retained<AnyObject> {
            self.children_for(item)[index as usize].object().into()
        }

        #[unsafe(method(outlineView:isItemExpandable:))]
        unsafe fn is_expandable(&self, _: &NSOutlineView, item: &AnyObject) -> bool {
            self.node_for(item).is_some_and(|node| !node.children.borrow().is_empty())
        }
    }

    unsafe impl NSOutlineViewDelegate for TreeHandler {
        #[unsafe(method_id(outlineView:viewForTableColumn:item:))]
        unsafe fn view_for_item(
            &self,
            _: &NSOutlineView,
            _: Option<&NSTableColumn>,
            item: &AnyObject,
        ) -> Option<Retained<NSView>> {
            self.node_for(item).map(|node| {
                let label = NSTextField::labelWithString(&NSString::from_str(&node.label.borrow()), self.mtm());
                label.setEnabled(self.ivars().enabled.get() && node.enabled.get());
                label.into_super().into_super()
            })
        }

        #[unsafe(method(outlineView:shouldSelectItem:))]
        unsafe fn should_select_item(&self, _: &NSOutlineView, item: &AnyObject) -> bool {
            self.ivars().enabled.get() && self.node_for(item).is_some_and(|node| node.enabled.get())
        }

        #[unsafe(method(outlineViewSelectionDidChange:))]
        fn selection_did_change(&self, notification: &NSNotification) {
            if self.ivars().suppress_selection_change.get() { return; }
            let Some(outline) = notification.object().and_then(|object| object.downcast::<NSOutlineView>().ok()) else { return; };
            if let Some(value) = self.value_at_row(&outline, outline.selectedRow())
                && let Some(callback) = self.ivars().on_value_change.get()
            { callback(&value); }
        }
    }

    impl TreeHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, sender: &NSOutlineView) {
            self.invoke_activation(sender);
        }
    }
);

impl TreeHandler {
    fn new(mtm: MainThreadMarker, state: TreeHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }

    fn all_nodes(&self) -> Vec<Rc<TreeNodeDefinition>> {
        fn append(output: &mut Vec<Rc<TreeNodeDefinition>>, nodes: &[Rc<TreeNodeDefinition>]) {
            for node in nodes {
                output.push(node.clone());
                append(output, &node.children.borrow());
            }
        }
        let mut output = Vec::new();
        append(&mut output, &self.ivars().roots.borrow());
        output
    }

    fn node_for(&self, item: &AnyObject) -> Option<Rc<TreeNodeDefinition>> {
        let value = item.downcast_ref::<NSString>()?.to_string();
        self.all_nodes()
            .into_iter()
            .find(|node| node.value.borrow().as_str() == value)
    }

    fn children_for(&self, item: Option<&AnyObject>) -> Vec<Rc<TreeNodeDefinition>> {
        item.and_then(|item| self.node_for(item))
            .map(|node| node.children.borrow().clone())
            .unwrap_or_else(|| self.ivars().roots.borrow().clone())
    }

    fn value_at_row(&self, outline: &NSOutlineView, row: isize) -> Option<String> {
        if !self.ivars().enabled.get() || row < 0 {
            return None;
        }
        let item = outline.itemAtRow(row)?;
        let node = self.node_for(&item)?;
        node.enabled.get().then(|| node.value.borrow().clone())
    }

    fn invoke_activation(&self, outline: &NSOutlineView) {
        let row = if outline.clickedRow() >= 0 {
            outline.clickedRow()
        } else {
            outline.selectedRow()
        };
        if let Some(value) = self.value_at_row(outline, row)
            && let Some(callback) = self.ivars().on_activate.get()
        {
            callback(&value);
        }
    }

    fn apply_selection(&self, outline: &NSOutlineView, value: Option<&str>) {
        if !self.ivars().enabled.get() {
            self.ivars().suppress_selection_change.set(true);
            unsafe { outline.deselectAll(None) };
            self.ivars().suppress_selection_change.set(false);
            return;
        }
        let row = value
            .and_then(|value| {
                self.all_nodes()
                    .into_iter()
                    .find(|node| node.enabled.get() && node.value.borrow().as_str() == value)
            })
            .map(|node| unsafe { outline.rowForItem(Some(&node.object())) })
            .filter(|row| *row >= 0);
        self.ivars().suppress_selection_change.set(true);
        if let Some(row) = row {
            outline.selectRowIndexes_byExtendingSelection(
                &NSIndexSet::indexSetWithIndex(row as usize),
                false,
            );
            outline.scrollRowToVisible(row);
        } else {
            unsafe { outline.deselectAll(None) };
        }
        self.ivars().suppress_selection_change.set(false);
    }
}

struct ActivatingOutlineViewState {
    handler: Retained<TreeHandler>,
}

define_class!(
    #[unsafe(super = NSOutlineView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActivatingOutlineViewState]
    struct ActivatingOutlineView;

    unsafe impl NSObjectProtocol for ActivatingOutlineView {}

    impl ActivatingOutlineView {
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if matches!(event.keyCode(), 36 | 76) {
                self.ivars().handler.invoke_activation(self);
            } else {
                unsafe { msg_send![super(self), keyDown: event] }
            }
        }
    }
);

impl ActivatingOutlineView {
    fn new(mtm: MainThreadMarker, handler: Retained<TreeHandler>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ActivatingOutlineViewState { handler });
        unsafe { msg_send![super(this), init] }
    }
}

#[component]
pub(crate) fn TreeViewHost(props: &TreeViewHostProps, element: &Element) -> Element {
    require_visual_mount!(element, TreeView, output);
    const CLASSES: [&str; 2] = ["__TreeView", "__appkit_TreeView"];
    let matched_styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &CLASSES,
    );
    let effective_style = resolved_view_style(matched_styles, &props.view);
    let roots = Rc::new(RefCell::new(Vec::new()));
    let (revision, set_revision) = create_state(0usize);
    let mtm = MainThreadMarker::new().expect("TreeView must mount on the main thread");
    let handler = TreeHandler::new(
        mtm,
        TreeHandlerState {
            roots: roots.clone(),
            on_value_change: props.on_value_change.clone(),
            on_activate: props.on_activate.clone(),
            enabled: props.enabled.clone(),
            suppress_selection_change: Cell::new(false),
        },
    );
    let outline = ActivatingOutlineView::new(mtm, handler.clone());
    configure_table(&outline);
    outline.setHeaderView(None);
    let column =
        NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &NSString::from_str("__tree"));
    column.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
    outline.addTableColumn(&column);
    unsafe {
        outline.setOutlineTableColumn(Some(&column));
    }
    unsafe {
        outline.setDataSource(Some(ProtocolObject::from_ref(&*handler)));
        outline.setDelegate(Some(ProtocolObject::from_ref(&*handler)));
        outline.setTarget(Some(&handler));
        outline.setDoubleAction(Some(sel!(activate:)));
    }
    let scroll = NSScrollView::new(mtm);
    scroll.setHasVerticalScroller(true);
    scroll.setDocumentView(Some(&outline));
    native_control::mount_with_intrinsic_size(
        element,
        scroll.clone().into_super(),
        effective_style.clone(),
        &props.view,
        revision.clone().into_readonly(),
        LogicalSize::new(240.0, 180.0),
    );
    let handler_id = nanoid::nanoid!();
    TREE_HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler.clone()));
    element.on_unmount(closure!(
        [handler_id] || {
            TREE_HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));
    scoped_effect!(
        [outline, handler, props.value, props.enabled, revision] || {
            let _ = revision.get();
            let expanded = (0..outline.numberOfRows())
                .filter_map(|row| outline.itemAtRow(row))
                .filter(|item| unsafe { outline.isItemExpanded(Some(item)) })
                .filter_map(|item| item.downcast::<NSString>().ok())
                .map(|item| item.to_string())
                .collect::<HashSet<_>>();
            outline.setEnabled(enabled.get());
            outline.reloadData();
            for node in handler.all_nodes() {
                if expanded.contains(node.value.borrow().as_str()) {
                    unsafe { outline.expandItem(Some(&node.object())) };
                }
            }
            handler.apply_selection(&outline, value.get().as_deref());
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

#[component]
pub(crate) fn TreeViewNodeHost(props: &CollectionNodeProps, element: &Element) -> Element {
    let parent = element
        .context::<TreeParentContext>()
        .expect("tree node must be beneath TreeView");
    let node = Rc::new(TreeNodeDefinition {
        registration_id: nanoid::nanoid!(),
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
            let next = value.get();
            if *node.value.borrow() != next {
                *node.value.borrow_mut() = next;
                *node.object.borrow_mut() = None;
                (parent.changed)();
            }
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
                TableViewColumn::new("role", "Role"),
            ]
        );
    }
}
