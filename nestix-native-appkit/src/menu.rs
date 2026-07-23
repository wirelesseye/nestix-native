use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{
    Element, Layout, PropValue, Shared, State, callback, closure, component,
    components::ContextProvider, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    CheckMenuItemProps, ContextMenuPosition, ContextMenuPresenter, ContextMenuProps,
    ContextMenuRegistration, MenuBarProps, MenuItemProps, MenuProps, MenuSeparatorProps,
    RadioMenuItemProps, Shortcut, ShortcutKey, ShortcutModifiers, SubmenuProps,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, rc::Retained,
    sel,
};
use objc2_app_kit::{
    NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags, NSMenu, NSMenuItem, NSView,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSString};

use crate::{root::RootContext, window::WindowContext};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<MenuItemHandler>>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct MenuContext {
    add: Shared<dyn Fn(&NSMenuItem)>,
    insert: Shared<dyn Fn(&NSMenuItem, usize)>,
    remove: Shared<dyn Fn(&NSMenuItem)>,
}

#[derive(Clone)]
pub(crate) struct ContextMenuContext {
    menu: State<Option<Retained<NSMenu>>>,
    target: State<Option<Shared<dyn Any>>>,
}

#[derive(Clone)]
pub(crate) struct TrayMenuContext {
    pub menu: State<Option<Retained<NSMenu>>>,
}

#[derive(Clone)]
struct MenuBarContext {
    menu: State<Option<Retained<NSMenu>>>,
}

fn menu_context(menu: &Retained<NSMenu>) -> MenuContext {
    MenuContext {
        add: callback!([menu] |item: &NSMenuItem| menu.addItem(item)),
        insert: callback!([menu] |item: &NSMenuItem, index: usize| {
            menu.insertItem_atIndex(item, index as _)
        }),
        remove: callback!([menu] |item: &NSMenuItem| {
            if menu.indexOfItem(item) >= 0 {
                menu.removeItem(item);
            }
        }),
    }
}

fn new_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::new());
    menu.setAutoenablesItems(false);
    menu
}

#[component]
pub fn Menu(props: &MenuProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let menu = new_menu(mtm);

    if let Some(context) = element.context::<MenuBarContext>() {
        context.menu.set(Some(menu.clone()));
        element.on_unmount(closure!(
            [context, menu] || {
                if contains_menu(&context.menu.get(), &menu) {
                    context.menu.set(None);
                }
            }
        ));
    } else if let Some(context) = element.context::<ContextMenuContext>() {
        context.menu.set(Some(menu.clone()));
        scoped_effect!(
            element,
            [element, context.target] || {
                if let Some(handle) = target.get()
                    && let Some(pointer) = handle.downcast_ref::<*const NSObject>()
                {
                    // Make the invisible menu subtree resolve to the wrapped
                    // view for placement of later visual siblings.
                    element.provide_handle(*pointer);
                }
            }
        );
        element.on_unmount(closure!(
            [context, menu] || {
                if context
                    .menu
                    .get()
                    .is_some_and(|current| std::ptr::eq::<NSMenu>(current.as_ref(), menu.as_ref()))
                {
                    context.menu.set(None);
                }
            }
        ));
    } else if let Some(context) = element.context::<TrayMenuContext>() {
        context.menu.set(Some(menu.clone()));
        element.on_unmount(closure!(
            [context, menu] || {
                if contains_menu(&context.menu.get(), &menu) {
                    context.menu.set(None);
                }
            }
        ));
    }

    layout! {
        ContextProvider<MenuContext>(menu_context(&menu)) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn MenuBar(props: &MenuBarProps, element: &Element) -> Element {
    let root = element.context::<RootContext>().unwrap();
    let window = element.context::<WindowContext>();
    let menu = create_state(None::<Retained<NSMenu>>);
    let registered = Rc::new(RefCell::new(None::<Retained<NSMenu>>));

    scoped_effect!(
        element,
        [root, window, menu, registered] || {
            let current = menu.get();
            if let Some(current) = current {
                registered.replace(Some(current.clone()));
                if let Some(window) = &window {
                    window.menu.set(Some(current.clone()));
                    if window.ns_window.isKeyWindow() {
                        root.active_window_menu.set(Some(current));
                    }
                } else {
                    root.app_menu.set(Some(current));
                }
            } else if let Some(previous) = registered.take() {
                unregister_menu(&root, window.as_deref(), &previous);
            }
        }
    );

    element.on_unmount(closure!(
        [root, window, registered] || {
            if let Some(previous) = registered.take() {
                unregister_menu(&root, window.as_deref(), &previous);
            }
        }
    ));

    layout! {
        ContextProvider<MenuBarContext>(MenuBarContext { menu }) {
            $(props.menu.clone().map(|menu| Layout::from(menu.clone())))
        }
    }
}

fn contains_menu(slot: &Option<Retained<NSMenu>>, menu: &NSMenu) -> bool {
    slot.as_ref()
        .is_some_and(|current| std::ptr::eq(current.as_ref(), menu))
}

fn unregister_menu(root: &RootContext, window: Option<&WindowContext>, menu: &NSMenu) {
    if let Some(window) = window {
        if contains_menu(&window.menu.get(), menu) {
            window.menu.set(None);
        }
        if contains_menu(&root.active_window_menu.get(), menu) {
            root.active_window_menu.set(None);
        }
    } else if contains_menu(&root.app_menu.get(), menu) {
        root.app_menu.set(None);
    }
}

#[component]
pub fn Submenu(props: &SubmenuProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let parent = element.context::<MenuContext>().unwrap();
    let submenu = new_menu(mtm);
    let item = new_item(&props.label.get(), None, mtm);
    item.setSubmenu(Some(&submenu));

    place_item(element, &parent, &item);
    scoped_effect!(
        element,
        [item, props.label, props.enabled, props.visible] || {
            item.setTitle(&NSString::from_str(&label.get()));
            item.setEnabled(enabled.get());
            item.setHidden(!visible.get());
        }
    );

    layout! {
        ContextProvider<MenuContext>(menu_context(&submenu)) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn MenuItem(props: &MenuItemProps, element: &Element) {
    let mtm = MainThreadMarker::new().unwrap();
    let parent = element.context::<MenuContext>().unwrap();
    let handler = MenuItemHandler::new(
        mtm,
        MenuItemHandlerState::Activate(props.on_activate.clone()),
    );
    let item = new_item(&props.label.get(), Some(&handler), mtm);
    retain_handler(element, handler);
    place_item(element, &parent, &item);
    update_common_item(
        element,
        &item,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
}

#[component]
pub fn CheckMenuItem(props: &CheckMenuItemProps, element: &Element) {
    let mtm = MainThreadMarker::new().unwrap();
    let parent = element.context::<MenuContext>().unwrap();
    let handler = MenuItemHandler::new(
        mtm,
        MenuItemHandlerState::Check(props.on_checked_change.clone()),
    );
    let item = new_item(&props.label.get(), Some(&handler), mtm);
    retain_handler(element, handler);
    place_item(element, &parent, &item);
    update_common_item(
        element,
        &item,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [item, props.checked] || {
            item.setState(if checked.get() {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    );
}

#[component]
pub fn RadioMenuItem(props: &RadioMenuItemProps, element: &Element) {
    let mtm = MainThreadMarker::new().unwrap();
    let parent = element.context::<MenuContext>().unwrap();
    let handler = MenuItemHandler::new(
        mtm,
        MenuItemHandlerState::Radio {
            group: props.group.clone(),
            on_select: props.on_select.clone(),
        },
    );
    let item = new_item(&props.label.get(), Some(&handler), mtm);
    retain_handler(element, handler);
    place_item(element, &parent, &item);
    update_common_item(
        element,
        &item,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [item, props.selected] || {
            item.setState(if selected.get() {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    );
    scoped_effect!(
        element,
        [item, props.group] || {
            let group = NSString::from_str(&group.get());
            unsafe { item.setRepresentedObject(Some(&group)) };
        }
    );
}

#[component]
pub fn MenuSeparator(props: &MenuSeparatorProps, element: &Element) {
    let mtm = MainThreadMarker::new().unwrap();
    let parent = element.context::<MenuContext>().unwrap();
    let item = NSMenuItem::separatorItem(mtm);
    place_item(element, &parent, &item);
    scoped_effect!(
        element,
        [item, props.visible] || {
            item.setHidden(!visible.get());
        }
    );
}

#[component]
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let context = Rc::new(ContextMenuContext {
        menu: create_state(None),
        target: create_state(None),
    });
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));

    scoped_effect!(
        element,
        [context, props.children] || {
            children.get().on_last_handle_change(closure!(
                [context] | handle | {
                    context.target.set(handle);
                }
            ));
        }
    );

    scoped_effect!(
        element,
        [context.menu, context.target, props.controller, registration] || {
            registration.borrow_mut().take();
            if let Some(handle) = target.get()
                && let Some(pointer) = handle.downcast_ref::<*const NSObject>()
            {
                let object = unsafe { &**pointer };
                if let Some(view) = object.downcast_ref::<NSView>() {
                    // NSResponder's menu property is an AppKit main-thread API.
                    let menu = menu.get();
                    unsafe { view.setMenu(menu.as_deref()) };

                    if let (Some(menu), Some(controller)) = (menu, controller.get()) {
                        let view = view.retain();
                        let presenter = ContextMenuPresenter {
                            show: callback!([menu, view] |position: ContextMenuPosition| {
                                let point = match position {
                                    ContextMenuPosition::Cursor => {
                                        let Some(window) = view.window() else {
                                            return false;
                                        };
                                        view.convertPoint_fromView(
                                            window.mouseLocationOutsideOfEventStream(),
                                            None,
                                        )
                                    }
                                    ContextMenuPosition::Anchor => {
                                        NSPoint::new(0.0, view.bounds().size.height)
                                    }
                                    ContextMenuPosition::Point(position) => NSPoint::new(
                                        position.x,
                                        view.bounds().size.height - position.y,
                                    ),
                                };
                                let _ = menu.popUpMenuPositioningItem_atLocation_inView(
                                    None,
                                    point,
                                    Some(&view),
                                );
                                // The return value describes how tracking ended,
                                // not whether the menu was presented. Cancelling
                                // the menu is still a successful show operation.
                                true
                            }),
                            dismiss: callback!([menu] || menu.cancelTracking()),
                        };
                        registration
                            .borrow_mut()
                            .replace(controller.bind(presenter));
                    }
                }
            }
        }
    );

    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));

    layout! {
        ContextProvider<ContextMenuContext>(context) [props.children, props.menu] {
            yield $(children.get())
            yield $(menu.get())
        }
    }
}

fn new_item(
    label: &str,
    handler: Option<&MenuItemHandler>,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(label),
            handler.map(|_| sel!(activate:)),
            &NSString::new(),
        )
    };
    unsafe { item.setTarget(handler.map(|handler| handler.as_ref())) };
    item
}

fn place_item(element: &Element, parent: &MenuContext, item: &Retained<NSMenuItem>) {
    element.on_place(closure!(
        [parent, item] | placement | {
            if let Some(index) = placement.index {
                (parent.remove)(&item);
                (parent.insert)(&item, index);
            } else {
                (parent.add)(&item);
            }
        }
    ));
    element.on_unmount(closure!([parent, item] || (parent.remove)(&item)));
}

fn retain_handler(element: &Element, handler: Retained<MenuItemHandler>) {
    let id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| {
        handlers.insert(id.clone(), handler);
    });
    element.on_unmount(move || {
        HANDLERS.with_borrow_mut(|handlers| handlers.remove(&id));
    });
}

fn update_common_item(
    element: &Element,
    item: &Retained<NSMenuItem>,
    label: PropValue<String>,
    enabled: PropValue<bool>,
    visible: PropValue<bool>,
    shortcut: PropValue<Option<Shortcut>>,
) {
    let item = item.clone();
    scoped_effect!(
        element,
        [item, label, enabled, visible, shortcut] || {
            item.setTitle(&NSString::from_str(&label.get()));
            item.setEnabled(enabled.get());
            item.setHidden(!visible.get());
            apply_shortcut(&item, shortcut.get());
        }
    );
}

fn apply_shortcut(item: &NSMenuItem, shortcut: Option<Shortcut>) {
    let Some(shortcut) = shortcut else {
        item.setKeyEquivalent(&NSString::new());
        item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
        return;
    };
    let key = match shortcut.key() {
        ShortcutKey::Character(value) => value.to_ascii_lowercase(),
        ShortcutKey::Backspace => '\u{8}',
        ShortcutKey::Delete => '\u{7f}',
        ShortcutKey::Down => '\u{f701}',
        ShortcutKey::End => '\u{f72b}',
        ShortcutKey::Enter => '\r',
        ShortcutKey::Escape => '\u{1b}',
        ShortcutKey::Home => '\u{f729}',
        ShortcutKey::Insert => '\u{f727}',
        ShortcutKey::Left => '\u{f702}',
        ShortcutKey::PageDown => '\u{f72d}',
        ShortcutKey::PageUp => '\u{f72c}',
        ShortcutKey::Right => '\u{f703}',
        ShortcutKey::Tab => '\t',
        ShortcutKey::Up => '\u{f700}',
        ShortcutKey::Function(number) => char::from_u32(0xf703 + number as u32).unwrap(),
    };
    let modifiers = shortcut.modifiers();
    let mut flags = NSEventModifierFlags::empty();
    if modifiers.contains(ShortcutModifiers::PRIMARY) {
        flags |= NSEventModifierFlags::Command;
    }
    if modifiers.contains(ShortcutModifiers::SHIFT) {
        flags |= NSEventModifierFlags::Shift;
    }
    if modifiers.contains(ShortcutModifiers::ALT) {
        flags |= NSEventModifierFlags::Option;
    }
    item.setKeyEquivalent(&NSString::from_str(&key.to_string()));
    item.setKeyEquivalentModifierMask(flags);
}

enum MenuItemHandlerState {
    Activate(PropValue<Option<Shared<dyn Fn()>>>),
    Check(PropValue<Option<Shared<dyn Fn(bool)>>>),
    Radio {
        group: PropValue<String>,
        on_select: PropValue<Option<Shared<dyn Fn()>>>,
    },
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixMenuItemHandler"]
    #[ivars = MenuItemHandlerState]
    struct MenuItemHandler;

    unsafe impl NSObjectProtocol for MenuItemHandler {}

    impl MenuItemHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, sender: &NSMenuItem) {
            match self.ivars() {
                MenuItemHandlerState::Activate(callback) => {
                    if let Some(callback) = callback.get() {
                        callback();
                    }
                }
                MenuItemHandlerState::Check(callback) => {
                    let checked = sender.state() != NSControlStateValueOn;
                    sender.setState(if checked {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                    if let Some(callback) = callback.get() {
                        callback(checked);
                    }
                }
                MenuItemHandlerState::Radio { group, on_select } => {
                    let group = group.get();
                    if let Some(menu) = unsafe { sender.menu() } {
                        for item in menu.itemArray().iter() {
                            let is_same_group = item
                                .representedObject()
                                .and_then(|value| value.downcast::<NSString>().ok())
                                .is_some_and(|value| value.to_string() == group);
                            if is_same_group {
                                item.setState(NSControlStateValueOff);
                            }
                        }
                    }
                    sender.setState(NSControlStateValueOn);
                    if let Some(callback) = on_select.get() {
                        callback();
                    }
                }
            }
        }
    }
);

impl MenuItemHandler {
    fn new(mtm: MainThreadMarker, state: MenuItemHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
