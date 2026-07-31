use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{
    Element, Layout, Shared, State, StateSetter, callback, closure, component,
    components::ContextProvider, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    ContextMenuPosition, ContextMenuPresenter, ContextMenuProps, ContextMenuRegistration,
    MenuBarProps, MenuEntryKind, MenuHostContext, MenuModel, Shortcut, ShortcutKey,
    ShortcutModifiers,
};

pub use nestix_native_core::{
    CheckMenuItem, CheckMenuItemProps, Menu, MenuItem, MenuItemProps, MenuProps, MenuSeparator,
    MenuSeparatorProps, RadioMenuItem, RadioMenuItemProps, Submenu, SubmenuProps,
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

#[derive(Clone)]
pub(crate) struct ContextMenuContext {
    menu: State<Option<Retained<NSMenu>>>,
    set_menu: StateSetter<Option<Retained<NSMenu>>>,
    target: State<Option<Shared<dyn Any>>>,
    set_target: StateSetter<Option<Shared<dyn Any>>>,
}

fn new_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::new());
    menu.setAutoenablesItems(false);
    menu
}

#[component]
pub fn MenuBar(props: &MenuBarProps, element: &Element) -> Element {
    let root = element.context::<RootContext>().unwrap();
    let window = element.context::<WindowContext>();
    let (menu, set_menu) = create_state(None::<Retained<NSMenu>>);
    let (description, set_description) = create_state(None::<MenuModel>);
    let handlers = Rc::new(RefCell::new(HashMap::new()));
    let registered = Rc::new(RefCell::new(None::<Retained<NSMenu>>));

    scoped_effect!(
        [description, set_menu, handlers] || {
            set_menu.set(
                description
                    .get()
                    .map(|model| render_menu_model(&model, &handlers)),
            );
        }
    );

    scoped_effect!(
        [root, window, menu, registered] || {
            let current = menu.get();
            if let Some(current) = current {
                registered.replace(Some(current.clone()));
                if let Some(window) = &window {
                    window.set_menu.set(Some(current.clone()));
                    if window.ns_window.isKeyWindow() {
                        root.set_active_window_menu.set(Some(current));
                    }
                } else {
                    root.set_app_menu.set(Some(current));
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
        ContextProvider<MenuHostContext>(
            MenuHostContext { menu: description, set_menu: set_description },
        ) {
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
            window.set_menu.set(None);
        }
        if contains_menu(&root.active_window_menu.get(), menu) {
            root.set_active_window_menu.set(None);
        }
    } else if contains_menu(&root.app_menu.get(), menu) {
        root.set_app_menu.set(None);
    }
}

#[component]
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let (menu, set_menu) = create_state(None);
    let (target, set_target) = create_state(None);
    let context = Rc::new(ContextMenuContext {
        menu,
        set_menu,
        target,
        set_target,
    });
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));
    let (description, set_description) = create_state(None::<MenuModel>);
    let handlers = Rc::new(RefCell::new(HashMap::new()));
    let set_native_menu = context.set_menu.clone();

    scoped_effect!(
        [description, set_native_menu, handlers] || {
            set_native_menu.set(
                description
                    .get()
                    .map(|model| render_menu_model(&model, &handlers)),
            );
        }
    );

    scoped_effect!(
        [context, props.children] || {
            children.get().on_last_handle_change(closure!(
                [context] | handle | {
                    context.set_target.set(handle);
                }
            ));
        }
    );

    scoped_effect!(
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
            yield ContextProvider<MenuHostContext>(
                MenuHostContext { menu: description.clone(), set_menu: set_description.clone() },
            ) {
                $(menu.get())
            }
        }
    }
}

pub(crate) fn render_menu_model(
    model: &MenuModel,
    handlers: &RefCell<HashMap<usize, Retained<MenuItemHandler>>>,
) -> Retained<NSMenu> {
    let mtm = MainThreadMarker::new().unwrap();
    let menu = new_menu(mtm);

    for entry in model.entries().into_iter().filter(|entry| entry.visible()) {
        match entry.kind() {
            MenuEntryKind::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
            MenuEntryKind::Submenu(submenu) => {
                let submenu = render_menu_model(&submenu, handlers);
                let item = new_item(&entry.label(), None, mtm);
                item.setEnabled(entry.enabled());
                item.setSubmenu(Some(&submenu));
                menu.addItem(&item);
            }
            MenuEntryKind::Item | MenuEntryKind::Check | MenuEntryKind::Radio => {
                let existing = handlers.borrow().get(&entry.id()).cloned();
                let handler = existing.unwrap_or_else(|| {
                    let activate: Shared<dyn Fn()> = callback!([entry] || entry.activate());
                    let handler =
                        MenuItemHandler::new(mtm, MenuItemHandlerState::Activate(activate));
                    handlers.borrow_mut().insert(entry.id(), handler.clone());
                    handler
                });
                let item = new_item(&entry.label(), Some(&handler), mtm);
                item.setEnabled(entry.enabled());
                if matches!(entry.kind(), MenuEntryKind::Check | MenuEntryKind::Radio) {
                    item.setState(if entry.checked() {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                }
                apply_shortcut(&item, entry.shortcut());
                menu.addItem(&item);
            }
        }
    }

    menu
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

pub(crate) enum MenuItemHandlerState {
    Activate(Shared<dyn Fn()>),
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixMenuItemHandler"]
    #[ivars = MenuItemHandlerState]
    pub(crate) struct MenuItemHandler;

    unsafe impl NSObjectProtocol for MenuItemHandler {}

    impl MenuItemHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, _sender: &NSMenuItem) {
            let MenuItemHandlerState::Activate(callback) = self.ivars();
            callback();
        }
    }
);

impl MenuItemHandler {
    fn new(mtm: MainThreadMarker, state: MenuItemHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
