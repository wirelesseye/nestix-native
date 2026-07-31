use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use nestix::{
    Element, PropValue, Shared, StateSetter, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::dpi::LogicalSize;
use nestix_native_core::{
    NavigationItemProps, SidebarNavigationProps, StyleContext, StyleScope, matched_style,
    resolved_view_style,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSControlTextEditingDelegate, NSLayoutConstraint, NSScrollView,
    NSTableColumn, NSTableColumnResizingOptions, NSTableView, NSTableViewColumnAutoresizingStyle,
    NSTableViewDataSource, NSTableViewDelegate, NSTableViewStyle, NSTextField, NSView,
};
use objc2_foundation::{NSArray, NSIndexSet, NSNotification, NSObject, NSObjectProtocol, NSString};

use crate::{native_control, sidebar::SidebarContext};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<NavigationHandler>>> =
        RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct NavigationItemDefinition {
    registration_id: String,
    label: PropValue<String>,
    value: PropValue<String>,
    enabled: PropValue<bool>,
}

#[derive(Clone)]
struct SidebarNavigationContext {
    items: Rc<RefCell<Vec<NavigationItemDefinition>>>,
    set_revision: StateSetter<usize>,
}

impl SidebarNavigationContext {
    fn changed(&self) {
        self.set_revision
            .mutate(|revision| *revision = revision.wrapping_add(1));
    }
}

struct NavigationHandlerState {
    items: Rc<RefCell<Vec<NavigationItemDefinition>>>,
    on_value_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
    suppress_selection_change: Cell<bool>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NavigationHandlerState]
    struct NavigationHandler;

    unsafe impl NSObjectProtocol for NavigationHandler {}

    unsafe impl NSControlTextEditingDelegate for NavigationHandler {}

    unsafe impl NSTableViewDataSource for NavigationHandler {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _: &NSTableView) -> isize {
            self.ivars().items.borrow().len() as isize
        }
    }

    unsafe impl NSTableViewDelegate for NavigationHandler {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(
            &self,
            _: &NSTableView,
            _: Option<&NSTableColumn>,
            row: isize,
        ) -> Option<Retained<NSView>> {
            let items = self.ivars().items.borrow();
            items.get(row as usize).map(|item| {
                let container = NSView::new(self.mtm());
                let label = NSTextField::labelWithString(
                    &NSString::from_str(&item.label.get()),
                    self.mtm(),
                );
                label.setEnabled(item.enabled.get());
                label.setTranslatesAutoresizingMaskIntoConstraints(false);
                container.addSubview(&label);
                NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
                    label
                        .centerYAnchor()
                        .constraintEqualToAnchor(&container.centerYAnchor()),
                    label
                        .leadingAnchor()
                        .constraintEqualToAnchor(&container.leadingAnchor()),
                    label
                        .trailingAnchor()
                        .constraintLessThanOrEqualToAnchor(&container.trailingAnchor()),
                ]));
                container
            })
        }

        #[unsafe(method(tableView:shouldSelectRow:))]
        fn should_select_row(&self, _: &NSTableView, row: isize) -> bool {
            self.ivars()
                .items
                .borrow()
                .get(row as usize)
                .is_some_and(|item| item.enabled.get())
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
            let row = table.selectedRow();
            if row < 0 {
                return;
            }
            let value = self
                .ivars()
                .items
                .borrow()
                .get(row as usize)
                .map(|item| item.value.get());
            if let (Some(value), Some(on_value_change)) =
                (value, self.ivars().on_value_change.get())
            {
                on_value_change(&value);
            }
        }
    }
);

impl NavigationHandler {
    fn new(mtm: MainThreadMarker, state: NavigationHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }

    fn apply_selection(&self, table: &NSTableView, value: Option<&str>) {
        let index = matching_item_index(&self.ivars().items.borrow(), value);
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

fn matching_item_index(items: &[NavigationItemDefinition], value: Option<&str>) -> Option<usize> {
    let value = value?;
    items
        .iter()
        .position(|item| item.value.get().as_str() == value)
}

#[component]
pub fn SidebarNavigation(props: &SidebarNavigationProps, element: &Element) -> Element {
    require_visual_mount!(element, SidebarNavigation, output);
    const DEFAULT_CLASSES: [&str; 2] = ["__SidebarNavigation", "__appkit_SidebarNavigation"];
    element
        .context::<SidebarContext>()
        .expect("SidebarNavigation must be mounted beneath Sidebar");

    let matched_styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched_styles, &props.view);
    let items = Rc::new(RefCell::new(Vec::new()));
    let (revision, set_revision) = create_state(0usize);

    let mtm =
        MainThreadMarker::new().expect("SidebarNavigation must be mounted on the main thread");
    let table = NSTableView::new(mtm);
    table.setStyle(NSTableViewStyle::SourceList);
    table.setHeaderView(None);
    table.setAllowsMultipleSelection(false);
    table.setAllowsEmptySelection(true);
    table.setAllowsColumnReordering(false);
    table.setAllowsColumnResizing(false);
    table.setColumnAutoresizingStyle(
        NSTableViewColumnAutoresizingStyle::LastColumnOnlyAutoresizingStyle,
    );
    table.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );

    let column = NSTableColumn::initWithIdentifier(
        NSTableColumn::alloc(mtm),
        &NSString::from_str("nestix.sidebar.navigation.label"),
    );
    column.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
    table.addTableColumn(&column);

    let handler = NavigationHandler::new(
        mtm,
        NavigationHandlerState {
            items: items.clone(),
            on_value_change: props.on_value_change.clone(),
            suppress_selection_change: Cell::new(false),
        },
    );
    unsafe {
        table.setDataSource(Some(ProtocolObject::from_ref(&*handler)));
        table.setDelegate(Some(ProtocolObject::from_ref(&*handler)));
    }

    let scroll = NSScrollView::new(mtm);
    scroll.setDrawsBackground(false);
    scroll.setHasVerticalScroller(true);
    scroll.setDocumentView(Some(&table));

    native_control::mount_with_intrinsic_size(
        element,
        scroll.clone().into_super(),
        effective_style.clone(),
        &props.view,
        revision.clone().into_readonly(),
        LogicalSize::new(160.0, 150.0),
    );

    let handler_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler.clone()));
    element.on_unmount(closure!(
        [handler_id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    scoped_effect!(
        [table, handler, props.value, revision] || {
            let _ = revision.get();
            table.reloadData();
            handler.apply_selection(&table, value.get().as_deref());
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<SidebarNavigationContext>(
                SidebarNavigationContext { items, set_revision,  },
            ) {
                $(props.children.clone())
            }
        }
    }
}

#[component]
pub fn NavigationItem(props: &NavigationItemProps, element: &Element) {
    require_visual_mount!(element, NavigationItem);
    let context = element
        .context::<SidebarNavigationContext>()
        .expect("NavigationItem must be mounted beneath SidebarNavigation");
    let registration_id = nanoid::nanoid!();
    let definition = NavigationItemDefinition {
        registration_id: registration_id.clone(),
        label: props.label.clone(),
        value: props.value.clone(),
        enabled: props.enabled.clone(),
    };

    element.on_place(closure!(
        [context, definition] | placement | {
            let mut items = context.items.borrow_mut();
            items.retain(|item| item.registration_id != definition.registration_id);
            let index = placement.index.unwrap_or(items.len()).min(items.len());
            items.insert(index, definition.clone());
            drop(items);
            context.changed();
        }
    ));

    element.on_unmount(closure!(
        [context, registration_id] || {
            context
                .items
                .borrow_mut()
                .retain(|item| item.registration_id != registration_id);
            context.changed();
        }
    ));

    scoped_effect!(
        [context, props.label, props.value, props.enabled] || {
            let _ = (label.get(), value.get(), enabled.get());
            context.changed();
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: &str) -> NavigationItemDefinition {
        NavigationItemDefinition {
            registration_id: value.to_owned(),
            label: PropValue::from_plain(value.to_owned()),
            value: PropValue::from_plain(value.to_owned()),
            enabled: PropValue::from_plain(true),
        }
    }

    #[test]
    fn controlled_selection_resolves_first_matching_value() {
        let items = [item("home"), item("settings"), item("settings")];
        assert_eq!(matching_item_index(&items, Some("settings")), Some(1));
        assert_eq!(matching_item_index(&items, Some("missing")), None);
        assert_eq!(matching_item_index(&items, None), None);
    }
}
