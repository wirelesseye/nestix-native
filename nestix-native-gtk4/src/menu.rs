use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use gtk4::{gio, glib, prelude::*};
use nestix::{
    Element, Layout, Shared, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
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

static NEXT_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_ITEM_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone)]
struct ContextMenuContext {
    menu: State<Option<Rc<MenuData>>>,
    target: State<Option<Shared<dyn Any>>>,
    actions: gio::SimpleActionGroup,
    action_prefix: String,
    revision: State<usize>,
}

#[derive(Clone)]
struct MenuBarContext {
    menu: State<Option<Rc<MenuData>>>,
    actions: gio::SimpleActionGroup,
    action_prefix: String,
    revision: State<usize>,
}

struct MenuData {
    model: gio::Menu,
    entries: RefCell<Vec<Rc<Entry>>>,
    actions: gio::SimpleActionGroup,
    action_prefix: String,
    revision: State<usize>,
}

impl PartialEq for MenuData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

struct Entry {
    kind: EntryKind,
    label: RefCell<String>,
    enabled: Cell<bool>,
    visible: Cell<bool>,
    checked: Cell<bool>,
    shortcut: Cell<Option<Shortcut>>,
}

enum EntryKind {
    Item {
        action_name: String,
        action: gio::SimpleAction,
    },
    Check {
        action_name: String,
        action: gio::SimpleAction,
    },
    Radio {
        action_name: String,
        target: String,
        action: gio::SimpleAction,
    },
    Submenu {
        menu: Rc<MenuData>,
        action_name: String,
        action: gio::SimpleAction,
    },
    Separator,
}

impl Entry {
    fn action(&self) -> Option<&gio::SimpleAction> {
        match &self.kind {
            EntryKind::Item { action, .. }
            | EntryKind::Check { action, .. }
            | EntryKind::Radio { action, .. }
            | EntryKind::Submenu { action, .. } => Some(action),
            EntryKind::Separator => None,
        }
    }

    fn action_name(&self) -> Option<&str> {
        match &self.kind {
            EntryKind::Item { action_name, .. }
            | EntryKind::Check { action_name, .. }
            | EntryKind::Radio { action_name, .. }
            | EntryKind::Submenu { action_name, .. } => Some(action_name),
            EntryKind::Separator => None,
        }
    }
}

impl MenuData {
    fn new(
        actions: gio::SimpleActionGroup,
        action_prefix: String,
        revision: State<usize>,
    ) -> Rc<Self> {
        Rc::new(Self {
            model: gio::Menu::new(),
            entries: RefCell::new(Vec::new()),
            actions,
            action_prefix,
            revision,
        })
    }

    fn rebuild(&self) {
        self.model.remove_all();
        let mut section = gio::Menu::new();
        let mut sections = Vec::new();

        for entry in self
            .entries
            .borrow()
            .iter()
            .filter(|entry| entry.visible.get())
        {
            if matches!(entry.kind, EntryKind::Separator) {
                if section.n_items() > 0 {
                    sections.push(section);
                    section = gio::Menu::new();
                }
                continue;
            }

            if let Some(action) = entry.action() {
                action.set_enabled(entry.enabled.get());
            }
            let label = entry.label.borrow();
            match &entry.kind {
                EntryKind::Submenu { menu, .. } => {
                    menu.rebuild();
                    let item = gio::MenuItem::new_submenu(Some(&label), &menu.model);
                    if let Some(action_name) = entry.action_name() {
                        item.set_action_and_target_value(
                            Some(&format!("{}.{}", self.action_prefix, action_name)),
                            None,
                        );
                    }
                    section.append_item(&item);
                }
                EntryKind::Item { .. } | EntryKind::Check { .. } => {
                    let detailed_action =
                        format!("{}.{}", self.action_prefix, entry.action_name().unwrap());
                    let item = gio::MenuItem::new(Some(&label), Some(&detailed_action));
                    if let Some(shortcut) = entry.shortcut.get() {
                        item.set_attribute_value(
                            "accel",
                            Some(&accelerator(shortcut).to_variant()),
                        );
                    }
                    section.append_item(&item);
                }
                EntryKind::Radio { target, .. } => {
                    let item = gio::MenuItem::new(Some(&label), None);
                    item.set_action_and_target_value(
                        Some(&format!(
                            "{}.{}",
                            self.action_prefix,
                            entry.action_name().unwrap()
                        )),
                        Some(&target.to_variant()),
                    );
                    if let Some(shortcut) = entry.shortcut.get() {
                        item.set_attribute_value(
                            "accel",
                            Some(&accelerator(shortcut).to_variant()),
                        );
                    }
                    section.append_item(&item);
                }
                EntryKind::Separator => unreachable!(),
            }
        }

        if section.n_items() > 0 {
            sections.push(section);
        }
        for section in sections {
            self.model.append_section(None, &section);
        }
        self.revision
            .mutate(|revision| *revision = revision.wrapping_add(1));
    }
}

fn render_menu_model(
    model: &MenuModel,
    actions: gio::SimpleActionGroup,
    action_prefix: String,
    revision: State<usize>,
) -> Rc<MenuData> {
    let menu = MenuData::new(actions, action_prefix, revision);
    let mut entries = Vec::new();
    for description in model.entries().into_iter().filter(|entry| entry.visible()) {
        let kind = match description.kind() {
            MenuEntryKind::Separator => EntryKind::Separator,
            MenuEntryKind::Submenu(submenu) => {
                let submenu = render_menu_model(
                    &submenu,
                    menu.actions.clone(),
                    menu.action_prefix.clone(),
                    menu.revision.clone(),
                );
                let (action_name, action) = new_action(&menu, false);
                EntryKind::Submenu {
                    menu: submenu,
                    action_name,
                    action,
                }
            }
            MenuEntryKind::Item => {
                let (action_name, action) = new_action(&menu, false);
                action.connect_activate(closure!([description] | _, _ | description.activate()));
                EntryKind::Item {
                    action_name,
                    action,
                }
            }
            MenuEntryKind::Check => {
                let (action_name, action) = new_action(&menu, true);
                action.set_state(&description.checked().to_variant());
                action.connect_activate(closure!([description] | _, _ | description.activate()));
                EntryKind::Check {
                    action_name,
                    action,
                }
            }
            MenuEntryKind::Radio => {
                let (action_name, target, action) = new_radio_action(&menu);
                action.set_state(
                    &if description.checked() {
                        target.clone()
                    } else {
                        String::new()
                    }
                    .to_variant(),
                );
                action.connect_activate(closure!([description] | _, _ | description.activate()));
                EntryKind::Radio {
                    action_name,
                    target,
                    action,
                }
            }
        };
        entries.push(Rc::new(Entry {
            kind,
            label: RefCell::new(description.label()),
            enabled: Cell::new(description.enabled()),
            visible: Cell::new(true),
            checked: Cell::new(description.checked()),
            shortcut: Cell::new(description.shortcut()),
        }));
    }
    *menu.entries.borrow_mut() = entries;
    menu.rebuild();
    menu
}

fn clear_actions(actions: &gio::SimpleActionGroup) {
    for name in actions.list_actions() {
        actions.remove_action(&name);
    }
}

#[component]
/// Installs a GTK menu bar on the containing window.
pub fn MenuBar(props: &MenuBarProps, element: &Element) -> Element {
    let context_id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
    let window = element.context::<crate::WindowContext>();
    let context = Rc::new(MenuBarContext {
        menu: create_state(None),
        actions: gio::SimpleActionGroup::new(),
        action_prefix: format!("nestix-menu-bar-{context_id}"),
        revision: create_state(0),
    });
    let description = create_state(None::<MenuModel>);
    let native_menu = context.menu.clone();
    let actions = context.actions.clone();
    let action_prefix = context.action_prefix.clone();
    let revision = context.revision.clone();
    scoped_effect!(
        [description, native_menu, actions, action_prefix, revision] || {
            clear_actions(&actions);
            native_menu.set(description.get().map(|model| {
                render_menu_model(
                    &model,
                    actions.clone(),
                    action_prefix.clone(),
                    revision.clone(),
                )
            }));
        }
    );
    let menu_bar = gtk4::PopoverMenuBar::from_model(None::<&gio::MenuModel>);
    let shortcuts = Rc::new(RefCell::new(None::<gtk4::ShortcutController>));

    if let Some(window) = &window {
        if let Some(previous) = window.menu_bar.replace(Some(menu_bar.clone())) {
            window.menu_bar_container.remove(&previous);
        }
        window
            .menu_bar_container
            .insert_child_after(&menu_bar, gtk4::Widget::NONE);
        window
            .window
            .insert_action_group(&context.action_prefix, Some(&context.actions));
        window.correct_content_size.set(true);
        element.provide_handle(menu_bar.clone());
    }

    scoped_effect!(
        [
            window,
            context,
            context.menu,
            context.revision,
            menu_bar,
            shortcuts
        ] || {
            let _ = revision.get();
            let menu = menu.get();
            menu_bar.set_menu_model(menu.as_ref().map(|menu| &menu.model));
            let Some(window) = &window else { return };
            window.correct_content_size.set(true);
            if let Some(previous) = shortcuts.borrow_mut().take() {
                window.window.remove_controller(&previous);
            }
            let controller = gtk4::ShortcutController::new();
            if let Some(menu) = &menu {
                add_shortcuts(&controller, menu);
            }
            window.window.add_controller(controller.clone());
            shortcuts.replace(Some(controller));
        }
    );

    element.on_unmount(closure!(
        [window, context, menu_bar, shortcuts] || {
            let Some(window) = &window else { return };
            if let Some(controller) = shortcuts.borrow_mut().take() {
                window.window.remove_controller(&controller);
            }
            window
                .window
                .insert_action_group(&context.action_prefix, gio::ActionGroup::NONE);
            let owns_slot = window
                .menu_bar
                .borrow()
                .as_ref()
                .is_some_and(|current| current == &menu_bar);
            if owns_slot {
                window.menu_bar.borrow_mut().take();
                window.menu_bar_container.remove(&menu_bar);
                window.correct_content_size.set(true);
            }
        }
    ));

    layout! {
        ContextProvider<MenuHostContext>(MenuHostContext { menu: description }) {
            $(props.menu.clone().map(|menu| Layout::from(menu.clone())))
        }
    }
}

#[component]
/// Attaches a GTK context menu to a visual element.
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let context_id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
    let context = Rc::new(ContextMenuContext {
        menu: create_state(None),
        target: create_state(None),
        actions: gio::SimpleActionGroup::new(),
        action_prefix: format!("nestix-context-{context_id}"),
        revision: create_state(0),
    });
    let description = create_state(None::<MenuModel>);
    let native_menu = context.menu.clone();
    let actions = context.actions.clone();
    let action_prefix = context.action_prefix.clone();
    let revision = context.revision.clone();
    scoped_effect!(
        [description, native_menu, actions, action_prefix, revision] || {
            clear_actions(&actions);
            native_menu.set(description.get().map(|model| {
                render_menu_model(
                    &model,
                    actions.clone(),
                    action_prefix.clone(),
                    revision.clone(),
                )
            }));
        }
    );
    let popover = gtk4::PopoverMenu::from_model(None::<&gio::MenuModel>);
    popover.set_has_arrow(false);
    let attached = Rc::new(RefCell::new(None::<AttachedTarget>));
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));
    let cursor = Rc::new(Cell::new(None::<(f64, f64)>));

    scoped_effect!(
        [context, props.children] || {
            children.get().on_last_handle_change(closure!(
                [context] | handle | {
                    context.target.set(handle);
                }
            ));
        }
    );

    scoped_effect!(
        [
            context,
            context.menu,
            context.target,
            context.revision,
            popover,
            attached,
            cursor
        ] || {
            detach_target(&popover, &attached);
            let menu = menu.get();
            popover.set_menu_model(menu.as_ref().map(|menu| &menu.model));
            let Some(handle) = target.get() else {
                return;
            };
            let Some(target) = handle.downcast_ref::<gtk4::Widget>().cloned() else {
                return;
            };
            let _ = revision.get();

            target.insert_action_group(&context.action_prefix, Some(&context.actions));
            popover.set_parent(&target);
            let motion = gtk4::EventControllerMotion::new();
            motion.connect_motion(closure!([cursor] | _, x, y | cursor.set(Some((x, y)))));
            target.add_controller(motion.clone());
            let gesture = gtk4::GestureClick::new();
            gesture.set_button(3);
            gesture.connect_pressed(closure!(
                [popover, cursor] | gesture,
                _,
                x,
                y | {
                    cursor.set(Some((x, y)));
                    point_popover(&popover, x, y);
                    popover.popup();
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                }
            ));
            target.add_controller(gesture.clone());
            let shortcuts = gtk4::ShortcutController::new();
            if let Some(menu) = &menu {
                add_shortcuts(&shortcuts, menu);
            }
            target.add_controller(shortcuts.clone());
            attached.replace(Some(AttachedTarget {
                widget: target,
                gesture,
                motion,
                shortcuts,
                action_prefix: context.action_prefix.clone(),
            }));
        }
    );

    scoped_effect!(
        [
            context.menu,
            context.target,
            props.controller,
            registration,
            popover,
            cursor
        ] || {
            registration.borrow_mut().take();
            let Some(controller) = controller.get() else {
                return;
            };
            let Some(handle) = target.get() else {
                return;
            };
            let Some(target) = handle.downcast_ref::<gtk4::Widget>().cloned() else {
                return;
            };
            if menu.get().is_none() {
                return;
            }
            let presenter = ContextMenuPresenter {
                show: callback!([popover, target, cursor] |position: ContextMenuPosition| {
                    if target.root().is_none() {
                        return false;
                    }
                    let (x, y) = match position {
                        ContextMenuPosition::Cursor => cursor
                            .get()
                            .unwrap_or((target.width() as f64 / 2.0, target.height() as f64 / 2.0)),
                        ContextMenuPosition::Anchor => (0.0, target.height() as f64),
                        ContextMenuPosition::Point(point) => (point.x, point.y),
                    };
                    point_popover(&popover, x, y);
                    popover.popup();
                    true
                }),
                dismiss: callback!([popover] || popover.popdown()),
            };
            registration
                .borrow_mut()
                .replace(controller.bind(presenter));
        }
    );

    element.on_unmount(closure!(
        [popover, attached, registration] || {
            registration.borrow_mut().take();
            detach_target(&popover, &attached);
        }
    ));

    layout! {
        ContextProvider<ContextMenuContext>(context) [props.children, props.menu] {
            yield $(children.get())
            yield ContextProvider<MenuHostContext>(MenuHostContext { menu: description.clone() }) {
                $(menu.get())
            }
        }
    }
}

struct AttachedTarget {
    widget: gtk4::Widget,
    gesture: gtk4::GestureClick,
    motion: gtk4::EventControllerMotion,
    shortcuts: gtk4::ShortcutController,
    action_prefix: String,
}

fn detach_target(popover: &gtk4::PopoverMenu, attached: &RefCell<Option<AttachedTarget>>) {
    if let Some(attached) = attached.borrow_mut().take() {
        attached.widget.remove_controller(&attached.gesture);
        attached.widget.remove_controller(&attached.motion);
        attached.widget.remove_controller(&attached.shortcuts);
        attached
            .widget
            .insert_action_group(&attached.action_prefix, gio::ActionGroup::NONE);
        popover.unparent();
    }
}

fn add_shortcuts(controller: &gtk4::ShortcutController, menu: &MenuData) {
    for entry in menu.entries.borrow().iter() {
        if !entry.visible.get() || !entry.enabled.get() {
            continue;
        }
        if let EntryKind::Submenu { menu, .. } = &entry.kind {
            add_shortcuts(controller, menu);
        }
        let (Some(shortcut), Some(action_name)) = (entry.shortcut.get(), entry.action_name())
        else {
            continue;
        };
        let Some(trigger) = gtk4::ShortcutTrigger::parse_string(&accelerator(shortcut)) else {
            continue;
        };
        let action = gtk4::NamedAction::new(&format!("{}.{}", menu.action_prefix, action_name));
        controller.add_shortcut(gtk4::Shortcut::new(Some(trigger), Some(action)));
    }
}

fn point_popover(popover: &gtk4::PopoverMenu, x: f64, y: f64) {
    popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
        x.round() as i32,
        y.round() as i32,
        1,
        1,
    )));
}

fn new_action(menu: &MenuData, stateful: bool) -> (String, gio::SimpleAction) {
    let name = format!("item-{}", NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed));
    let action = if stateful {
        gio::SimpleAction::new_stateful(&name, None, &false.to_variant())
    } else {
        gio::SimpleAction::new(&name, None)
    };
    menu.actions.add_action(&action);
    (name, action)
}

fn new_radio_action(menu: &MenuData) -> (String, String, gio::SimpleAction) {
    let name = format!("item-{}", NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed));
    let target = name.clone();
    let action = gio::SimpleAction::new_stateful(
        &name,
        Some(glib::VariantTy::STRING),
        &String::new().to_variant(),
    );
    menu.actions.add_action(&action);
    (name, target, action)
}

fn accelerator(shortcut: Shortcut) -> String {
    let mut value = String::new();
    let modifiers = shortcut.modifiers();
    if modifiers.contains(ShortcutModifiers::PRIMARY) {
        value.push_str("<Control>");
    }
    if modifiers.contains(ShortcutModifiers::SHIFT) {
        value.push_str("<Shift>");
    }
    if modifiers.contains(ShortcutModifiers::ALT) {
        value.push_str("<Alt>");
    }
    value.push_str(match shortcut.key() {
        ShortcutKey::Character(character) => return format!("{value}{character}"),
        ShortcutKey::Backspace => "BackSpace",
        ShortcutKey::Delete => "Delete",
        ShortcutKey::Down => "Down",
        ShortcutKey::End => "End",
        ShortcutKey::Enter => "Return",
        ShortcutKey::Escape => "Escape",
        ShortcutKey::Home => "Home",
        ShortcutKey::Insert => "Insert",
        ShortcutKey::Left => "Left",
        ShortcutKey::PageDown => "Page_Down",
        ShortcutKey::PageUp => "Page_Up",
        ShortcutKey::Right => "Right",
        ShortcutKey::Tab => "Tab",
        ShortcutKey::Up => "Up",
        ShortcutKey::Function(number) => return format!("{value}F{number}"),
    });
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_use_gtk_accelerator_syntax() {
        assert_eq!(accelerator(Shortcut::primary('O')), "<Control>O");
        assert_eq!(
            accelerator(
                Shortcut::new(
                    ShortcutKey::PageDown,
                    ShortcutModifiers::PRIMARY | ShortcutModifiers::SHIFT,
                )
                .unwrap()
            ),
            "<Control><Shift>Page_Down"
        );
    }
}
