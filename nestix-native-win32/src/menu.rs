use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

use nestix::{
    Element, Shared, StateSetter, callback, closure, component, components::ContextProvider,
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
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::ScreenToClient,
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
                MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, PostMessageW,
                SetForegroundWindow, SetMenu, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN,
                TrackPopupMenu, WM_CONTEXTMENU, WM_NULL,
            },
        },
    },
    core::HSTRING,
};

use crate::surface::{VisualHandle, visual_handle};

const SUBCLASS_ID: usize = 0x4e_65_73_74_69_78;

thread_local! {
    static TARGETS: RefCell<HashMap<*mut std::ffi::c_void, Vec<(VisualHandle, Weak<MenuData>)>>> = RefCell::new(HashMap::new());
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

pub(crate) struct MenuData {
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
}

#[derive(Clone)]
struct ContextMenuContext {
    set_target: StateSetter<Option<Shared<dyn Any>>>,
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

pub(crate) fn render_menu_model(model: &MenuModel, popup: bool) -> Rc<MenuData> {
    let menu = new_menu(popup);
    let entries = model
        .entries()
        .into_iter()
        .filter(|entry| entry.visible())
        .map(|description| {
            let kind = match description.kind() {
                MenuEntryKind::Separator => EntryKind::Separator,
                MenuEntryKind::Submenu(submenu) => {
                    EntryKind::Submenu(render_menu_model(&submenu, true))
                }
                MenuEntryKind::Item | MenuEntryKind::Check | MenuEntryKind::Radio => {
                    EntryKind::Item {
                        id: description.id(),
                        action: callback!([description] || description.activate()),
                    }
                }
            };
            Rc::new(Entry {
                kind,
                label: RefCell::new(description.label()),
                enabled: Cell::new(description.enabled()),
                visible: Cell::new(true),
                checked: Cell::new(description.checked()),
                shortcut: Cell::new(description.shortcut()),
            })
        })
        .collect();
    *menu.entries.borrow_mut() = entries;
    menu.rebuild();
    menu
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
        let Some(action) = self.action_for(id) else {
            return false;
        };
        action();
        true
    }

    fn action_for(&self, id: usize) -> Option<Shared<dyn Fn()>> {
        for entry in self.entries.borrow().iter() {
            match &entry.kind {
                EntryKind::Item {
                    id: entry_id,
                    action,
                } if *entry_id == id => {
                    return Some(action.clone());
                }
                EntryKind::Submenu(menu) => {
                    if let Some(action) = menu.action_for(id) {
                        return Some(action);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn activate_shortcut(&self, key: usize, modifiers: ShortcutModifiers) -> bool {
        let Some(action) = self.action_for_shortcut(key, modifiers) else {
            return false;
        };
        action();
        true
    }

    fn action_for_shortcut(
        &self,
        key: usize,
        modifiers: ShortcutModifiers,
    ) -> Option<Shared<dyn Fn()>> {
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
                    return Some(action.clone());
                }
                EntryKind::Submenu(menu) => {
                    if let Some(action) = menu.action_for_shortcut(key, modifiers) {
                        return Some(action);
                    }
                }
                _ => {}
            }
        }
        None
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
/// Installs a native Win32 menu bar on the containing window.
pub fn MenuBar(props: &MenuBarProps, element: &Element) -> Element {
    let window = element.context::<crate::WindowContext>();
    let (menu, set_menu) = create_state(None::<Rc<MenuData>>);
    let (description, set_description) = create_state(None::<MenuModel>);
    let attached = Rc::new(RefCell::new(None::<Rc<MenuData>>));

    scoped_effect!(
        [description] || {
            set_menu.set(
                description
                    .get()
                    .map(|model| render_menu_model(&model, false)),
            );
        }
    );

    scoped_effect!(
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
        ContextProvider<MenuHostContext>(
            MenuHostContext { menu: description, set_menu: set_description },
        ) {
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

pub(crate) fn show_tray_menu(menu: &MenuData, target: HWND, point: POINT) -> bool {
    menu.rebuild();
    unsafe {
        let _ = SetForegroundWindow(target);
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
        let _ = PostMessageW(Some(target), WM_NULL, WPARAM(0), LPARAM(0));
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
        let mut point = POINT {
            x: (lparam.0 as i16) as i32,
            y: ((lparam.0 >> 16) as i16) as i32,
        };
        let keyboard = point.x == -1 && point.y == -1;
        if !keyboard {
            let _ = unsafe { ScreenToClient(hwnd, &mut point) };
        }
        let menu = TARGETS.with_borrow(|targets| {
            targets
                .get(&hwnd.0)?
                .iter()
                .rev()
                .find_map(|(visual, menu)| {
                    (keyboard || visual.contains_client_point(point))
                        .then(|| menu.upgrade())
                        .flatten()
                })
        });
        if let Some(menu) = menu {
            show_menu(&menu, hwnd, ContextMenuPosition::Cursor);
            return LRESULT(0);
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[component]
/// Attaches a native Win32 context menu to a visual element.
pub fn ContextMenu(props: &ContextMenuProps, element: &Element) -> Element {
    let (menu, set_menu) = create_state(None::<Rc<MenuData>>);
    let (target, set_target) = create_state(None::<Shared<dyn Any>>);
    let registration = Rc::new(RefCell::new(None::<ContextMenuRegistration>));
    let (description, set_description) = create_state(None::<MenuModel>);
    let set_native_menu = set_menu.clone();
    scoped_effect!(
        [description, set_native_menu] || {
            set_native_menu.set(
                description
                    .get()
                    .map(|model| render_menu_model(&model, true)),
            );
        }
    );
    let registered_target = Rc::new(RefCell::new(None::<(HWND, Weak<MenuData>)>));
    let context = Rc::new(ContextMenuContext { set_target });
    scoped_effect!(
        [context, props.children] || {
            children.get().on_last_handle_change(closure!(
                [context] | handle | context.set_target.set(handle)
            ));
        }
    );
    scoped_effect!(
        [
            menu,
            target,
            props.controller,
            registration,
            registered_target
        ] || {
            registration.borrow_mut().take();
            if let Some((old, old_menu)) = registered_target.borrow_mut().take() {
                let last = TARGETS.with_borrow_mut(|targets| {
                    let Some(entries) = targets.get_mut(&old.0) else {
                        return false;
                    };
                    entries.retain(|(_, menu)| !Weak::ptr_eq(menu, &old_menu));
                    if entries.is_empty() {
                        targets.remove(&old.0);
                        true
                    } else {
                        false
                    }
                });
                if last {
                    unsafe {
                        let _ = RemoveWindowSubclass(old, Some(context_subclass), SUBCLASS_ID);
                    }
                }
            }
            if let (Some(menu), Some(handle)) = (menu.get(), target.get())
                && let Some(visual) = visual_handle(&handle)
            {
                let hwnd = visual.hwnd();
                let weak_menu = Rc::downgrade(&menu);
                let first = TARGETS.with_borrow_mut(|targets| {
                    let entries = targets.entry(hwnd.0).or_default();
                    let first = entries.is_empty();
                    entries.push((visual, weak_menu.clone()));
                    first
                });
                if first {
                    unsafe {
                        let _ = SetWindowSubclass(hwnd, Some(context_subclass), SUBCLASS_ID, 0);
                    }
                }
                registered_target.borrow_mut().replace((hwnd, weak_menu));
                if let Some(controller) = controller.get() {
                    registration
                        .borrow_mut()
                        .replace(controller.bind(ContextMenuPresenter {
                            show: callback!([menu] | position | show_menu(&menu, hwnd, position)),
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
        [registration, registered_target] || {
            registration.borrow_mut().take();
            if let Some((hwnd, menu)) = registered_target.borrow_mut().take() {
                let last = TARGETS.with_borrow_mut(|targets| {
                    let Some(entries) = targets.get_mut(&hwnd.0) else {
                        return false;
                    };
                    entries.retain(|(_, entry)| !Weak::ptr_eq(entry, &menu));
                    if entries.is_empty() {
                        targets.remove(&hwnd.0);
                        true
                    } else {
                        false
                    }
                });
                if last {
                    unsafe {
                        let _ = RemoveWindowSubclass(hwnd, Some(context_subclass), SUBCLASS_ID);
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_releases_menu_borrows_before_running_action() {
        let menu = new_menu(true);
        let submenu = new_menu(true);
        let weak_menu = Rc::downgrade(&menu);
        let weak_submenu = Rc::downgrade(&submenu);

        submenu.entries.borrow_mut().push(Rc::new(Entry {
            kind: EntryKind::Item {
                id: 1,
                action: callback!(
                    [weak_menu, weak_submenu] || {
                        weak_menu.upgrade().unwrap().entries.borrow_mut().clear();
                        weak_submenu.upgrade().unwrap().entries.borrow_mut().clear();
                    }
                ),
            },
            label: RefCell::new("Quit".into()),
            enabled: Cell::new(true),
            visible: Cell::new(true),
            checked: Cell::new(false),
            shortcut: Cell::new(None),
        }));
        menu.entries.borrow_mut().push(Rc::new(Entry {
            kind: EntryKind::Submenu(submenu.clone()),
            label: RefCell::new("Application".into()),
            enabled: Cell::new(true),
            visible: Cell::new(true),
            checked: Cell::new(false),
            shortcut: Cell::new(None),
        }));

        assert!(menu.activate(1));
        assert!(menu.entries.borrow().is_empty());
        assert!(submenu.entries.borrow().is_empty());
    }
}
