use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::atomic::{AtomicUsize, Ordering},
};

use nestix::{
    Element, PropValue, Shared, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    CheckMenuItemProps, ContextMenuPosition, ContextMenuPresenter, ContextMenuProps,
    ContextMenuRegistration, MenuBarProps, MenuItemProps, MenuProps, MenuSeparatorProps,
    RadioMenuItemProps, Shortcut, ShortcutKey, ShortcutModifiers, SubmenuProps,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyState, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1,
                VK_HOME, VK_INSERT, VK_LEFT, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
                VK_SHIFT, VK_TAB, VK_UP,
            },
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                AppendMenuW, CreateMenu, CreatePopupMenu, DestroyMenu, DrawMenuBar, EndMenu,
                GetCursorPos, GetWindowRect, HMENU, MF_BYPOSITION, MF_CHECKED, MF_DISABLED,
                MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SetMenu, TPM_LEFTALIGN,
                TPM_RETURNCMD, TPM_TOPALIGN, TrackPopupMenu, WM_CONTEXTMENU,
            },
        },
    },
    core::HSTRING,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
const SUBCLASS_ID: usize = 0x4e_65_73_74_69_78;

thread_local! {
    static TARGETS: RefCell<HashMap<*mut std::ffi::c_void, Weak<MenuData>>> = RefCell::new(HashMap::new());
    static MENU_BARS: RefCell<HashMap<*mut std::ffi::c_void, Weak<MenuData>>> = RefCell::new(HashMap::new());
}

struct NativeMenu(HMENU);
impl Drop for NativeMenu {
    fn drop(&mut self) {
        unsafe {
            // DestroyMenu recursively destroys attached submenus. Detach every
            // entry first because submenu handles are owned by their MenuData.
            while windows::Win32::UI::WindowsAndMessaging::RemoveMenu(self.0, 0, MF_BYPOSITION)
                .is_ok()
            {}
            let _ = DestroyMenu(self.0);
        }
    }
}

struct MenuData {
    native: NativeMenu,
    entries: RefCell<Vec<Rc<Entry>>>,
}

impl PartialEq for MenuData {
    fn eq(&self, other: &Self) -> bool {
        self.native.0 == other.native.0
    }
}

enum EntryKind {
    Item { id: usize, action: Shared<dyn Fn()> },
    Separator,
    Submenu(Rc<MenuData>),
}

struct Entry {
    kind: EntryKind,
    label: RefCell<String>,
    enabled: Cell<bool>,
    visible: Cell<bool>,
    checked: Cell<bool>,
    shortcut: Cell<Option<Shortcut>>,
    group: RefCell<Option<String>>,
}

#[derive(Clone)]
struct MenuContext(Rc<MenuData>);

#[derive(Clone)]
struct ContextMenuContext {
    menu: State<Option<Rc<MenuData>>>,
    target: State<Option<Shared<dyn Any>>>,
}

#[derive(Clone)]
struct MenuBarContext {
    menu: State<Option<Rc<MenuData>>>,
}

fn new_menu(popup: bool) -> Rc<MenuData> {
    Rc::new(MenuData {
        native: NativeMenu(unsafe {
            if popup {
                CreatePopupMenu()
            } else {
                CreateMenu()
            }
            .unwrap()
        }),
        entries: RefCell::new(Vec::new()),
    })
}

impl MenuData {
    fn rebuild(&self) {
        unsafe {
            while windows::Win32::UI::WindowsAndMessaging::RemoveMenu(
                self.native.0,
                0,
                MF_BYPOSITION,
            )
            .is_ok()
            {}
            for entry in self
                .entries
                .borrow()
                .iter()
                .filter(|entry| entry.visible.get())
            {
                let mut flags = match entry.kind {
                    EntryKind::Separator => MF_SEPARATOR,
                    _ => MF_STRING,
                };
                if !entry.enabled.get() {
                    flags |= MF_DISABLED | MF_GRAYED;
                }
                if entry.checked.get() {
                    flags |= MF_CHECKED;
                } else {
                    flags |= MF_UNCHECKED;
                }
                match &entry.kind {
                    EntryKind::Separator => {
                        let _ = AppendMenuW(self.native.0, flags, 0, None);
                    }
                    EntryKind::Item { id, .. } => {
                        let text = HSTRING::from(display_label(
                            &entry.label.borrow(),
                            entry.shortcut.get(),
                        ));
                        let _ = AppendMenuW(self.native.0, flags, *id, &text);
                    }
                    EntryKind::Submenu(submenu) => {
                        submenu.rebuild();
                        let text = HSTRING::from(entry.label.borrow().as_str());
                        let _ = AppendMenuW(
                            self.native.0,
                            flags | MF_POPUP,
                            submenu.native.0.0 as usize,
                            &text,
                        );
                    }
                }
            }
            MENU_BARS.with_borrow(|bars| {
                for (hwnd, menu) in bars {
                    if menu
                        .upgrade()
                        .as_deref()
                        .is_some_and(|menu| std::ptr::eq(menu, self))
                    {
                        let _ = DrawMenuBar(HWND(*hwnd));
                    }
                }
            });
        }
    }

    fn activate(&self, id: usize) -> bool {
        for entry in self.entries.borrow().iter() {
            match &entry.kind {
                EntryKind::Item {
                    id: entry_id,
                    action,
                } if *entry_id == id => {
                    action();
                    return true;
                }
                EntryKind::Submenu(menu) if menu.activate(id) => return true,
                _ => {}
            }
        }
        false
    }

    fn activate_shortcut(&self, key: usize, modifiers: ShortcutModifiers) -> bool {
        for entry in self.entries.borrow().iter() {
            if !entry.visible.get() || !entry.enabled.get() {
                continue;
            }
            match &entry.kind {
                EntryKind::Item { action, .. }
                    if entry.shortcut.get().is_some_and(|shortcut| {
                        shortcut.modifiers() == modifiers
                            && shortcut_key_code(shortcut.key()) == Some(key)
                    }) =>
                {
                    action();
                    return true;
                }
                EntryKind::Submenu(menu) if menu.activate_shortcut(key, modifiers) => return true,
                _ => {}
            }
        }
        false
    }
}

fn shortcut_key_code(key: ShortcutKey) -> Option<usize> {
    Some(match key {
        ShortcutKey::Character(value) if value.is_ascii_alphanumeric() => {
            value.to_ascii_uppercase() as usize
        }
        ShortcutKey::Character(_) => return None,
        ShortcutKey::Backspace => VK_BACK.0 as usize,
        ShortcutKey::Delete => VK_DELETE.0 as usize,
        ShortcutKey::Down => VK_DOWN.0 as usize,
        ShortcutKey::End => VK_END.0 as usize,
        ShortcutKey::Enter => VK_RETURN.0 as usize,
        ShortcutKey::Escape => VK_ESCAPE.0 as usize,
        ShortcutKey::Home => VK_HOME.0 as usize,
        ShortcutKey::Insert => VK_INSERT.0 as usize,
        ShortcutKey::Left => VK_LEFT.0 as usize,
        ShortcutKey::PageDown => VK_NEXT.0 as usize,
        ShortcutKey::PageUp => VK_PRIOR.0 as usize,
        ShortcutKey::Right => VK_RIGHT.0 as usize,
        ShortcutKey::Tab => VK_TAB.0 as usize,
        ShortcutKey::Up => VK_UP.0 as usize,
        ShortcutKey::Function(number) => VK_F1.0 as usize + number as usize - 1,
    })
}

fn display_label(label: &str, shortcut: Option<Shortcut>) -> String {
    shortcut.map_or_else(
        || label.to_owned(),
        |shortcut| format!("{label}\t{}", shortcut_text(shortcut)),
    )
}

fn shortcut_text(shortcut: Shortcut) -> String {
    let mut text = String::new();
    let modifiers = shortcut.modifiers();
    if modifiers.contains(ShortcutModifiers::PRIMARY) {
        text.push_str("Ctrl+");
    }
    if modifiers.contains(ShortcutModifiers::SHIFT) {
        text.push_str("Shift+");
    }
    if modifiers.contains(ShortcutModifiers::ALT) {
        text.push_str("Alt+");
    }
    text.push_str(&match shortcut.key() {
        ShortcutKey::Character(value) => value.to_ascii_uppercase().to_string(),
        ShortcutKey::Backspace => "Backspace".into(),
        ShortcutKey::Delete => "Del".into(),
        ShortcutKey::Down => "Down".into(),
        ShortcutKey::End => "End".into(),
        ShortcutKey::Enter => "Enter".into(),
        ShortcutKey::Escape => "Esc".into(),
        ShortcutKey::Home => "Home".into(),
        ShortcutKey::Insert => "Ins".into(),
        ShortcutKey::Left => "Left".into(),
        ShortcutKey::PageDown => "PgDn".into(),
        ShortcutKey::PageUp => "PgUp".into(),
        ShortcutKey::Right => "Right".into(),
        ShortcutKey::Tab => "Tab".into(),
        ShortcutKey::Up => "Up".into(),
        ShortcutKey::Function(number) => format!("F{number}"),
    });
    text
}

#[component]
pub fn Menu(props: &MenuProps, element: &Element) -> Element {
    let menu_bar = element.context::<MenuBarContext>();
    let menu = new_menu(menu_bar.is_none());
    if let Some(context) = menu_bar {
        context.menu.set(Some(menu.clone()));
        element.on_unmount(closure!(
            [context, menu] || {
                if context
                    .menu
                    .get()
                    .as_ref()
                    .is_some_and(|value| Rc::ptr_eq(value, &menu))
                {
                    context.menu.set(None);
                }
            }
        ));
    } else if let Some(context) = element.context::<ContextMenuContext>() {
        context.menu.set(Some(menu.clone()));
        element.on_unmount(closure!(
            [context, menu] || {
                if context
                    .menu
                    .get()
                    .as_ref()
                    .is_some_and(|value| Rc::ptr_eq(value, &menu))
                {
                    context.menu.set(None);
                }
            }
        ));
    }
    layout! {
        ContextProvider<MenuContext>(MenuContext(menu)) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn MenuBar(props: &MenuBarProps, element: &Element) -> Element {
    let window = element.context::<crate::WindowContext>();
    let menu = create_state(None::<Rc<MenuData>>);
    let attached = Rc::new(RefCell::new(None::<Rc<MenuData>>));

    scoped_effect!(
        element,
        [window, menu, attached] || {
            let Some(window) = &window else { return };
            if let Some(previous) = attached.take() {
                detach_menu_bar(window.hwnd, &previous);
            }
            if let Some(current) = menu.get() {
                current.rebuild();
                MENU_BARS.with_borrow_mut(|bars| {
                    bars.insert(window.hwnd.0, Rc::downgrade(&current));
                });
                unsafe {
                    let _ = SetMenu(window.hwnd, Some(current.native.0));
                    let _ = DrawMenuBar(window.hwnd);
                }
                attached.replace(Some(current));
            }
        }
    );

    element.on_unmount(closure!(
        [window, attached] || {
            if let Some(window) = &window
                && let Some(previous) = attached.take()
            {
                detach_menu_bar(window.hwnd, &previous);
            }
        }
    ));

    layout! {
        ContextProvider<MenuBarContext>(MenuBarContext { menu }) {
            $(props.menu.clone().map(|menu| nestix::Layout::from(menu.clone())))
        }
    }
}

pub(crate) fn handle_menu_command(hwnd: HWND, id: usize) {
    let menu = MENU_BARS.with_borrow(|bars| bars.get(&hwnd.0).and_then(Weak::upgrade));
    if let Some(menu) = menu {
        menu.activate(id);
    }
}

pub(crate) fn handle_menu_shortcut(hwnd: HWND, key: usize) -> bool {
    let Some(menu) = MENU_BARS.with_borrow(|bars| bars.get(&hwnd.0).and_then(Weak::upgrade)) else {
        return false;
    };
    let mut modifiers = ShortcutModifiers::NONE;
    unsafe {
        if GetKeyState(VK_CONTROL.0 as i32) < 0 {
            modifiers |= ShortcutModifiers::PRIMARY;
        }
        if GetKeyState(VK_SHIFT.0 as i32) < 0 {
            modifiers |= ShortcutModifiers::SHIFT;
        }
        if GetKeyState(VK_MENU.0 as i32) < 0 {
            modifiers |= ShortcutModifiers::ALT;
        }
    }
    menu.activate_shortcut(key, modifiers)
}

fn detach_menu_bar(hwnd: HWND, menu: &Rc<MenuData>) {
    let owns_slot = MENU_BARS.with_borrow(|bars| {
        bars.get(&hwnd.0)
            .and_then(Weak::upgrade)
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, menu))
    });
    if owns_slot {
        MENU_BARS.with_borrow_mut(|bars| {
            bars.remove(&hwnd.0);
        });
        unsafe {
            let _ = SetMenu(hwnd, None);
            let _ = DrawMenuBar(hwnd);
        }
    }
}

fn place_entry(element: &Element, menu: Rc<MenuData>, entry: Rc<Entry>) {
    element.on_place(closure!(
        [menu, entry] | placement | {
            let mut entries = menu.entries.borrow_mut();
            entries.retain(|current| !Rc::ptr_eq(current, &entry));
            let index = placement.index.unwrap_or(entries.len()).min(entries.len());
            entries.insert(index, entry.clone());
            drop(entries);
            menu.rebuild();
        }
    ));
    element.on_unmount(closure!(
        [menu, entry] || {
            menu.entries
                .borrow_mut()
                .retain(|current| !Rc::ptr_eq(current, &entry));
            menu.rebuild();
        }
    ));
}

fn common_effects(
    element: &Element,
    menu: Rc<MenuData>,
    entry: Rc<Entry>,
    label: PropValue<String>,
    enabled: PropValue<bool>,
    visible: PropValue<bool>,
    shortcut: PropValue<Option<Shortcut>>,
) {
    scoped_effect!(
        element,
        [menu, entry, label, enabled, visible, shortcut] || {
            *entry.label.borrow_mut() = label.get();
            entry.enabled.set(enabled.get());
            entry.visible.set(visible.get());
            entry.shortcut.set(shortcut.get());
            menu.rebuild();
        }
    );
}

#[component]
pub fn Submenu(props: &SubmenuProps, element: &Element) -> Element {
    let parent = element.context::<MenuContext>().unwrap().0.clone();
    let submenu = new_menu(true);
    let entry = Rc::new(Entry {
        kind: EntryKind::Submenu(submenu.clone()),
        label: RefCell::new(props.label.get()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        checked: Cell::new(false),
        shortcut: Cell::new(None),
        group: RefCell::new(None),
    });
    place_entry(element, parent.clone(), entry.clone());
    common_effects(
        element,
        parent,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        PropValue::from_plain(None),
    );
    layout! {
        ContextProvider<MenuContext>(MenuContext(submenu)) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn MenuItem(props: &MenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry = Rc::new(Entry {
        kind: EntryKind::Item {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            action: callback!(
                [props.on_activate] || {
                    if let Some(action) = on_activate.get() {
                        action();
                    }
                }
            ),
        },
        label: RefCell::new(props.label.get()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        checked: Cell::new(false),
        shortcut: Cell::new(None),
        group: RefCell::new(None),
    });
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        element,
        menu,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
}

#[component]
pub fn CheckMenuItem(props: &CheckMenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry_slot = Rc::new(RefCell::new(Weak::<Entry>::new()));
    let entry = Rc::new(Entry {
        kind: EntryKind::Item {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            action: callback!(
                [entry_slot, props.on_checked_change] || {
                    if let Some(entry) = entry_slot.borrow().upgrade() {
                        let checked = !entry.checked.get();
                        entry.checked.set(checked);
                        if let Some(action) = on_checked_change.get() {
                            action(checked);
                        }
                    }
                }
            ),
        },
        label: RefCell::new(props.label.get()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        checked: Cell::new(props.checked.get()),
        shortcut: Cell::new(None),
        group: RefCell::new(None),
    });
    *entry_slot.borrow_mut() = Rc::downgrade(&entry);
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        element,
        menu.clone(),
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [menu, entry, props.checked] || {
            entry.checked.set(checked.get());
            menu.rebuild();
        }
    );
}

#[component]
pub fn RadioMenuItem(props: &RadioMenuItemProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry_slot = Rc::new(RefCell::new(Weak::<Entry>::new()));
    let entry = Rc::new(Entry {
        kind: EntryKind::Item {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            action: callback!(
                [menu, entry_slot, props.group, props.on_select] || {
                    if let Some(selected) = entry_slot.borrow().upgrade() {
                        for item in menu.entries.borrow().iter() {
                            if !Rc::ptr_eq(item, &selected)
                                && item.checked.get()
                                && item.group.borrow().as_deref() == Some(group.get().as_str())
                            {
                                item.checked.set(false);
                            }
                        }
                        selected.checked.set(true);
                        if let Some(action) = on_select.get() {
                            action();
                        }
                        menu.rebuild();
                    }
                }
            ),
        },
        label: RefCell::new(props.label.get()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        checked: Cell::new(props.selected.get()),
        shortcut: Cell::new(None),
        group: RefCell::new(Some(props.group.get())),
    });
    *entry_slot.borrow_mut() = Rc::downgrade(&entry);
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        element,
        menu.clone(),
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        element,
        [menu, entry, props.selected] || {
            entry.checked.set(selected.get());
            menu.rebuild();
        }
    );
    scoped_effect!(
        element,
        [entry, props.group] || {
            *entry.group.borrow_mut() = Some(group.get());
        }
    );
}

#[component]
pub fn MenuSeparator(props: &MenuSeparatorProps, element: &Element) {
    let menu = element.context::<MenuContext>().unwrap().0.clone();
    let entry = Rc::new(Entry {
        kind: EntryKind::Separator,
        label: RefCell::new(String::new()),
        enabled: Cell::new(true),
        visible: Cell::new(props.visible.get()),
        checked: Cell::new(false),
        shortcut: Cell::new(None),
        group: RefCell::new(None),
    });
    place_entry(element, menu.clone(), entry.clone());
    scoped_effect!(
        element,
        [menu, entry, props.visible] || {
            entry.visible.set(visible.get());
            menu.rebuild();
        }
    );
}

fn show_menu(menu: &MenuData, target: HWND, position: ContextMenuPosition) -> bool {
    menu.rebuild();
    let mut point = POINT::default();
    unsafe {
        match position {
            ContextMenuPosition::Cursor => {
                if GetCursorPos(&mut point).is_err() {
                    return false;
                }
            }
            ContextMenuPosition::Anchor => {
                let mut rect = Default::default();
                if GetWindowRect(target, &mut rect).is_err() {
                    return false;
                }
                point.x = rect.left;
                point.y = rect.bottom;
            }
            ContextMenuPosition::Point(value) => {
                let mut rect = Default::default();
                if GetWindowRect(target, &mut rect).is_err() {
                    return false;
                }
                point.x = rect.left + value.x.round() as i32;
                point.y = rect.top + value.y.round() as i32;
            }
        }
        let id = TrackPopupMenu(
            menu.native.0,
            TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            Some(0),
            target,
            None,
        )
        .0 as usize;
        if id != 0 {
            menu.activate(id);
        }
        true
    }
}

unsafe extern "system" fn context_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_CONTEXTMENU {
        let menu = TARGETS.with_borrow(|targets| targets.get(&hwnd.0).and_then(Weak::upgrade));
        if let Some(menu) = menu {
            show_menu(&menu, hwnd, ContextMenuPosition::Cursor);
            return LRESULT(0);
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[component]
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let menu = create_state(None::<Rc<MenuData>>);
    let target = create_state(None::<Shared<dyn Any>>);
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));
    let registered_hwnd = Rc::new(Cell::new(None::<HWND>));
    let context = Rc::new(ContextMenuContext {
        menu: menu.clone(),
        target: target.clone(),
    });
    scoped_effect!(
        element,
        [context, props.children] || {
            children
                .get()
                .on_last_handle_change(closure!([context] | handle | context.target.set(handle)));
        }
    );
    scoped_effect!(
        element,
        [
            menu,
            target,
            props.controller,
            registration,
            registered_hwnd
        ] || {
            registration.borrow_mut().take();
            if let Some(old) = registered_hwnd.take() {
                TARGETS.with_borrow_mut(|targets| {
                    targets.remove(&old.0);
                });
                unsafe {
                    let _ = RemoveWindowSubclass(old, Some(context_subclass), SUBCLASS_ID);
                }
            }
            if let (Some(menu), Some(handle)) = (menu.get(), target.get())
                && let Some(hwnd) = handle.downcast_ref::<HWND>()
            {
                TARGETS.with_borrow_mut(|targets| {
                    targets.insert(hwnd.0, Rc::downgrade(&menu));
                });
                unsafe {
                    let _ = SetWindowSubclass(*hwnd, Some(context_subclass), SUBCLASS_ID, 0);
                }
                registered_hwnd.set(Some(*hwnd));
                if let Some(controller) = controller.get() {
                    registration
                        .borrow_mut()
                        .replace(controller.bind(ContextMenuPresenter {
                            show: callback!(
                                [menu, hwnd] | position | show_menu(&menu, hwnd, position)
                            ),
                            dismiss: callback!(
                                [] || unsafe {
                                    let _ = EndMenu();
                                }
                            ),
                        }));
                }
            }
        }
    );
    element.on_unmount(closure!(
        [registration, registered_hwnd] || {
            registration.borrow_mut().take();
            if let Some(hwnd) = registered_hwnd.take() {
                TARGETS.with_borrow_mut(|targets| {
                    targets.remove(&hwnd.0);
                });
                unsafe {
                    let _ = RemoveWindowSubclass(hwnd, Some(context_subclass), SUBCLASS_ID);
                }
            }
        }
    ));
    layout! {
        ContextProvider<ContextMenuContext>(context) [props.children, props.menu] {
            yield $(children.get())
            yield $(menu.get())
        }
    }
}
