use std::{
    cell::{Cell, RefCell},
    fmt,
    ops::{BitOr, BitOrAssign},
    rc::{Rc, Weak},
    sync::atomic::{AtomicUsize, Ordering},
};

use nestix::{
    Element, Layout, PropValue, Shared, State, StateSetter, callback, closure, component,
    components::ContextProvider, create_state, layout, props, scoped_effect,
};

static NEXT_MENU_ENTRY_ID: AtomicUsize = AtomicUsize::new(1);

/// A key which can be used in a native menu shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    Character(char),
    Backspace,
    Delete,
    Down,
    End,
    Enter,
    Escape,
    Home,
    Insert,
    Left,
    PageDown,
    PageUp,
    Right,
    Tab,
    Up,
    Function(u8),
}

impl ShortcutKey {
    /// Creates a printable ASCII shortcut key.
    pub fn character(value: char) -> Result<Self, InvalidShortcutKey> {
        if value.is_ascii() && !value.is_ascii_control() {
            Ok(Self::Character(value))
        } else {
            Err(InvalidShortcutKey)
        }
    }

    /// Creates an F1 through F24 shortcut key.
    pub fn function(number: u8) -> Result<Self, InvalidShortcutKey> {
        if (1..=24).contains(&number) {
            Ok(Self::Function(number))
        } else {
            Err(InvalidShortcutKey)
        }
    }

    pub fn is_valid(self) -> bool {
        match self {
            Self::Character(value) => value.is_ascii() && !value.is_ascii_control(),
            Self::Function(number) => (1..=24).contains(&number),
            _ => true,
        }
    }
}

/// Returned when a shortcut character is not printable ASCII or a function
/// key is outside F1 through F24.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidShortcutKey;

impl fmt::Display for InvalidShortcutKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("shortcut keys must be printable ASCII, a navigation key, or F1-F24")
    }
}

impl std::error::Error for InvalidShortcutKey {}

/// Location used when presenting a context menu imperatively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMenuPosition {
    /// Present at the current pointer location.
    Cursor,
    /// Present at the context menu's wrapped view.
    Anchor,
    /// Present at a logical point relative to the wrapped view's top-left.
    Point(dpi::LogicalPosition<f64>),
}

/// Error returned by an imperative context-menu operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuError {
    /// The controller is not associated with a mounted context menu.
    NotMounted,
    /// The native backend could not begin menu presentation.
    PresentationFailed,
}

impl fmt::Display for ContextMenuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted => formatter.write_str("context menu is not mounted"),
            Self::PresentationFailed => formatter.write_str("context menu could not be shown"),
        }
    }
}

impl std::error::Error for ContextMenuError {}

#[doc(hidden)]
#[derive(Clone)]
pub struct ContextMenuPresenter {
    pub show: Shared<dyn Fn(ContextMenuPosition) -> bool>,
    pub dismiss: Shared<dyn Fn()>,
}

#[derive(Default)]
struct ContextMenuControllerState {
    next_binding_id: u64,
    presenter: Option<(u64, ContextMenuPresenter)>,
}

/// Cloneable handle for presenting or dismissing a mounted context menu.
///
/// A controller is intended to be associated with one [`ContextMenuProps`] at
/// a time. Calls return [`ContextMenuError::NotMounted`] until that component
/// has mounted and again after it unmounts.
#[derive(Clone, Default)]
pub struct ContextMenuController {
    state: Rc<RefCell<ContextMenuControllerState>>,
}

impl fmt::Debug for ContextMenuController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContextMenuController")
            .field("mounted", &self.state.borrow().presenter.is_some())
            .finish()
    }
}

impl ContextMenuController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&self, position: ContextMenuPosition) -> Result<(), ContextMenuError> {
        let presenter = self
            .state
            .borrow()
            .presenter
            .as_ref()
            .map(|(_, presenter)| presenter.clone())
            .ok_or(ContextMenuError::NotMounted)?;
        (presenter.show)(position)
            .then_some(())
            .ok_or(ContextMenuError::PresentationFailed)
    }

    pub fn dismiss(&self) -> Result<(), ContextMenuError> {
        let presenter = self
            .state
            .borrow()
            .presenter
            .as_ref()
            .map(|(_, presenter)| presenter.clone())
            .ok_or(ContextMenuError::NotMounted)?;
        (presenter.dismiss)();
        Ok(())
    }

    #[doc(hidden)]
    pub fn bind(&self, presenter: ContextMenuPresenter) -> ContextMenuRegistration {
        let mut state = self.state.borrow_mut();
        let binding_id = state.next_binding_id;
        state.next_binding_id = state.next_binding_id.wrapping_add(1);
        state.presenter = Some((binding_id, presenter));
        ContextMenuRegistration {
            controller: self.clone(),
            binding_id,
        }
    }
}

#[doc(hidden)]
pub struct ContextMenuRegistration {
    controller: ContextMenuController,
    binding_id: u64,
}

impl Drop for ContextMenuRegistration {
    fn drop(&mut self) {
        let mut state = self.controller.state.borrow_mut();
        if state
            .presenter
            .as_ref()
            .is_some_and(|(binding_id, _)| *binding_id == self.binding_id)
        {
            state.presenter = None;
        }
    }
}

/// Platform-neutral shortcut modifiers. `PRIMARY` is Command on macOS and
/// Control on Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ShortcutModifiers(u8);

impl ShortcutModifiers {
    pub const NONE: Self = Self(0);
    pub const PRIMARY: Self = Self(1 << 0);
    pub const SHIFT: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for ShortcutModifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ShortcutModifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, nestix::InspectableValue)]
pub struct Shortcut {
    key: ShortcutKey,
    modifiers: ShortcutModifiers,
}

impl Shortcut {
    pub const PRIMARY: ShortcutModifiers = ShortcutModifiers::PRIMARY;
    pub const SHIFT: ShortcutModifiers = ShortcutModifiers::SHIFT;
    pub const ALT: ShortcutModifiers = ShortcutModifiers::ALT;

    pub fn new(key: ShortcutKey, modifiers: ShortcutModifiers) -> Result<Self, InvalidShortcutKey> {
        key.is_valid()
            .then_some(Self { key, modifiers })
            .ok_or(InvalidShortcutKey)
    }

    /// Convenience constructor for the usual application-command shortcut.
    /// Panics if `key` is not printable ASCII.
    pub fn primary(key: char) -> Self {
        Self::new(
            ShortcutKey::character(key).expect("invalid primary shortcut key"),
            ShortcutModifiers::PRIMARY,
        )
        .expect("invalid primary shortcut key")
    }

    pub const fn key(self) -> ShortcutKey {
        self.key
    }
    pub const fn modifiers(self) -> ShortcutModifiers {
        self.modifiers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn shortcut_keys_are_validated() {
        assert_eq!(ShortcutKey::character('A'), Ok(ShortcutKey::Character('A')));
        assert!(ShortcutKey::character('\n').is_err());
        assert!(ShortcutKey::character('é').is_err());
        assert_eq!(ShortcutKey::function(24), Ok(ShortcutKey::Function(24)));
        assert!(ShortcutKey::function(0).is_err());
        assert!(ShortcutKey::function(25).is_err());
    }

    #[test]
    fn modifiers_can_be_combined() {
        let shortcut = Shortcut::new(
            ShortcutKey::Character('S'),
            ShortcutModifiers::PRIMARY | ShortcutModifiers::SHIFT,
        )
        .unwrap();

        assert!(shortcut.modifiers().contains(ShortcutModifiers::PRIMARY));
        assert!(shortcut.modifiers().contains(ShortcutModifiers::SHIFT));
        assert!(!shortcut.modifiers().contains(ShortcutModifiers::ALT));
    }

    #[test]
    fn context_menu_controller_tracks_its_registration() {
        let controller = ContextMenuController::new();
        let shown = Rc::new(Cell::new(None));
        let dismissed = Rc::new(Cell::new(false));

        assert_eq!(
            controller.show(ContextMenuPosition::Anchor),
            Err(ContextMenuError::NotMounted)
        );

        let registration = controller.bind(ContextMenuPresenter {
            show: Shared::from({
                let shown = shown.clone();
                Rc::new(move |position| {
                    shown.set(Some(position));
                    true
                }) as Rc<dyn Fn(ContextMenuPosition) -> bool>
            }),
            dismiss: Shared::from({
                let dismissed = dismissed.clone();
                Rc::new(move || dismissed.set(true)) as Rc<dyn Fn()>
            }),
        });

        assert_eq!(controller.show(ContextMenuPosition::Cursor), Ok(()));
        assert_eq!(shown.get(), Some(ContextMenuPosition::Cursor));
        assert_eq!(controller.dismiss(), Ok(()));
        assert!(dismissed.get());

        drop(registration);
        assert_eq!(controller.dismiss(), Err(ContextMenuError::NotMounted));
    }

    #[test]
    fn stale_registration_does_not_clear_a_new_presenter() {
        fn presenter() -> ContextMenuPresenter {
            ContextMenuPresenter {
                show: Shared::from(Rc::new(|_: ContextMenuPosition| true)
                    as Rc<dyn Fn(ContextMenuPosition) -> bool>),
                dismiss: Shared::from(Rc::new(|| {}) as Rc<dyn Fn()>),
            }
        }

        let controller = ContextMenuController::new();
        let old = controller.bind(presenter());
        let _current = controller.bind(presenter());
        drop(old);

        assert_eq!(controller.show(ContextMenuPosition::Anchor), Ok(()));
    }
}

/// Properties for a native menu.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuProps {
    /// Menu items and submenus in display order.
    #[props(default)]
    pub children: Layout,
}

/// Presents a [`Menu`] as a window menu bar at this position in the visual
/// tree.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuBarProps {
    /// Menu element installed as the window's menu bar.
    pub menu: Element,
}

/// Properties for a submenu.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SubmenuProps {
    /// User-visible submenu label.
    #[props(start)]
    pub label: String,
    /// Whether the submenu accepts interaction.
    #[props(default = true)]
    pub enabled: bool,
    /// Whether the submenu is displayed.
    #[props(default = true)]
    pub visible: bool,
    /// Items contained by the submenu.
    #[props(default)]
    pub children: Layout,
}

/// Properties for an actionable menu item.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuItemProps {
    /// User-visible item label.
    #[props(start)]
    pub label: String,
    /// Whether the item accepts interaction.
    #[props(default = true)]
    pub enabled: bool,
    /// Whether the item is displayed.
    #[props(default = true)]
    pub visible: bool,
    /// Optional keyboard shortcut.
    pub shortcut: Option<Shortcut>,
    /// Called when the user activates the item.
    pub on_activate: Option<Shared<dyn Fn()>>,
}

/// Properties for a checkable menu item.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct CheckMenuItemProps {
    /// User-visible item label.
    #[props(start)]
    pub label: String,
    /// Whether the item accepts interaction.
    #[props(default = true)]
    pub enabled: bool,
    /// Whether the item is displayed.
    #[props(default = true)]
    pub visible: bool,
    /// Optional keyboard shortcut.
    pub shortcut: Option<Shortcut>,
    /// Whether the item is checked.
    #[props(default)]
    pub checked: bool,
    /// Called with the new checked state after activation.
    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}

/// Properties for an item in a mutually exclusive radio group.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct RadioMenuItemProps {
    /// User-visible item label.
    #[props(start)]
    pub label: String,
    /// Whether the item accepts interaction.
    #[props(default = true)]
    pub enabled: bool,
    /// Whether the item is displayed.
    #[props(default = true)]
    pub visible: bool,
    /// Optional keyboard shortcut.
    pub shortcut: Option<Shortcut>,
    /// Whether the item is selected.
    #[props(default)]
    pub selected: bool,
    /// Identifier shared by mutually exclusive items.
    pub group: String,
    /// Called when the user selects the item.
    pub on_select: Option<Shared<dyn Fn()>>,
}

/// Properties for a visual separator between menu items.
#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct MenuSeparatorProps {
    /// Whether the separator is displayed.
    #[props(default = true)]
    pub visible: bool,
}

/// The platform-neutral kind of a descriptive menu entry.
#[doc(hidden)]
#[derive(Clone)]
pub enum MenuEntryKind {
    Item,
    Check,
    Radio,
    Submenu(MenuModel),
    Separator,
}

struct MenuEntryData {
    id: usize,
    kind: MenuEntryKind,
    label: RefCell<String>,
    enabled: Cell<bool>,
    visible: Cell<bool>,
    checked: Cell<bool>,
    shortcut: Cell<Option<Shortcut>>,
    group: RefCell<Option<String>>,
    activate: RefCell<Option<Shared<dyn Fn()>>>,
}

/// A snapshot-capable, reactive description of one menu entry.
#[doc(hidden)]
#[derive(Clone)]
pub struct MenuEntry(Rc<MenuEntryData>);

impl MenuEntry {
    pub fn id(&self) -> usize {
        self.0.id
    }

    pub fn kind(&self) -> MenuEntryKind {
        self.0.kind.clone()
    }

    pub fn label(&self) -> String {
        self.0.label.borrow().clone()
    }

    pub fn enabled(&self) -> bool {
        self.0.enabled.get()
    }

    pub fn visible(&self) -> bool {
        self.0.visible.get()
    }

    pub fn checked(&self) -> bool {
        self.0.checked.get()
    }

    pub fn shortcut(&self) -> Option<Shortcut> {
        self.0.shortcut.get()
    }

    pub fn activate(&self) {
        if self.enabled()
            && self.visible()
            && let Some(activate) = self.0.activate.borrow().clone()
        {
            activate();
        }
    }
}

struct MenuModelData {
    entries: RefCell<Vec<MenuEntry>>,
    revision: State<usize>,
    set_revision: StateSetter<usize>,
}

/// Reactive platform-neutral description produced by [`Menu`].
#[doc(hidden)]
#[derive(Clone)]
pub struct MenuModel(Rc<MenuModelData>);

impl PartialEq for MenuModel {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl MenuModel {
    fn new() -> Self {
        let (revision, set_revision) = create_state(0);
        Self(Rc::new(MenuModelData {
            entries: RefCell::new(Vec::new()),
            revision,
            set_revision,
        }))
    }

    pub fn entries(&self) -> Vec<MenuEntry> {
        let _ = self.0.revision.get();
        self.0.entries.borrow().clone()
    }

    pub fn revision(&self) -> State<usize> {
        self.0.revision.clone()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    fn changed(&self) {
        self.0
            .set_revision
            .mutate(|revision| *revision = revision.wrapping_add(1));
    }
}

/// Context installed by backend-owned menu presenters.
#[doc(hidden)]
#[derive(Clone)]
pub struct MenuHostContext {
    pub menu: State<Option<MenuModel>>,
    pub set_menu: StateSetter<Option<MenuModel>>,
}

#[derive(Clone)]
struct MenuModelContext(MenuModel);

fn place_entry(element: &Element, menu: MenuModel, entry: MenuEntry) {
    element.on_place(closure!(
        [menu, entry] | placement | {
            let mut entries = menu.0.entries.borrow_mut();
            entries.retain(|current| !Rc::ptr_eq(&current.0, &entry.0));
            let index = placement.index.unwrap_or(entries.len()).min(entries.len());
            entries.insert(index, entry.clone());
            drop(entries);
            menu.changed();
        }
    ));
    element.on_unmount(closure!(
        [menu, entry] || {
            menu.0
                .entries
                .borrow_mut()
                .retain(|current| !Rc::ptr_eq(&current.0, &entry.0));
            menu.changed();
        }
    ));
}

fn common_effects(
    menu: MenuModel,
    entry: MenuEntry,
    label: PropValue<String>,
    enabled: PropValue<bool>,
    visible: PropValue<bool>,
    shortcut: PropValue<Option<Shortcut>>,
) {
    scoped_effect!(
        [menu, entry, label, enabled, visible, shortcut] || {
            *entry.0.label.borrow_mut() = label.get();
            entry.0.enabled.set(enabled.get());
            entry.0.visible.set(visible.get());
            entry.0.shortcut.set(shortcut.get());
            menu.changed();
        }
    );
}

fn new_entry(kind: MenuEntryKind) -> MenuEntry {
    MenuEntry(Rc::new(MenuEntryData {
        id: NEXT_MENU_ENTRY_ID.fetch_add(1, Ordering::Relaxed),
        kind,
        label: RefCell::new(String::new()),
        enabled: Cell::new(true),
        visible: Cell::new(true),
        checked: Cell::new(false),
        shortcut: Cell::new(None),
        group: RefCell::new(None),
        activate: RefCell::new(None),
    }))
}

/// Groups descriptive menu entries without selecting a native backend.
#[component]
pub fn Menu(props: &MenuProps, element: &Element) -> Element {
    let host = element
        .context::<MenuHostContext>()
        .expect("Menu must be contained by MenuBar, ContextMenu, or a menu host");
    let menu = MenuModel::new();
    host.set_menu.set(Some(menu.clone()));
    element.on_unmount(closure!(
        [host, menu] || {
            if host
                .menu
                .get()
                .as_ref()
                .is_some_and(|current| current.ptr_eq(&menu))
            {
                host.set_menu.set(None);
            }
        }
    ));

    layout! {
        ContextProvider<MenuModelContext>(MenuModelContext(menu)) {
            $(props.children.clone())
        }
    }
}

/// Adds a descriptive submenu without selecting a native backend.
#[component]
pub fn Submenu(props: &SubmenuProps, element: &Element) -> Element {
    let parent = element.context::<MenuModelContext>().unwrap().0.clone();
    let submenu = MenuModel::new();
    let entry = new_entry(MenuEntryKind::Submenu(submenu.clone()));
    place_entry(element, parent.clone(), entry.clone());
    common_effects(
        parent,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        PropValue::from_plain(None),
    );

    layout! {
        ContextProvider<MenuModelContext>(MenuModelContext(submenu)) {
            $(props.children.clone())
        }
    }
}

/// Adds a descriptive actionable menu item.
#[component]
pub fn MenuItem(props: &MenuItemProps, element: &Element) {
    let menu = element.context::<MenuModelContext>().unwrap().0.clone();
    let entry = new_entry(MenuEntryKind::Item);
    entry.0.activate.replace(Some(callback!(
        [props.on_activate] || {
            if let Some(activate) = on_activate.get() {
                activate();
            }
        }
    )));
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        menu,
        entry,
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
}

/// Adds a descriptive checkable menu item.
#[component]
pub fn CheckMenuItem(props: &CheckMenuItemProps, element: &Element) {
    let menu = element.context::<MenuModelContext>().unwrap().0.clone();
    let entry = new_entry(MenuEntryKind::Check);
    let weak_entry: Weak<MenuEntryData> = Rc::downgrade(&entry.0);
    entry.0.activate.replace(Some(callback!(
        [menu, weak_entry, props.on_checked_change] || {
            if let Some(entry) = weak_entry.upgrade() {
                let checked = !entry.checked.get();
                entry.checked.set(checked);
                menu.changed();
                if let Some(change) = on_checked_change.get() {
                    change(checked);
                }
            }
        }
    )));
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        menu.clone(),
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        [menu, entry, props.checked] || {
            entry.0.checked.set(checked.get());
            menu.changed();
        }
    );
}

/// Adds a descriptive mutually exclusive menu item.
#[component]
pub fn RadioMenuItem(props: &RadioMenuItemProps, element: &Element) {
    let menu = element.context::<MenuModelContext>().unwrap().0.clone();
    let entry = new_entry(MenuEntryKind::Radio);
    let weak_entry: Weak<MenuEntryData> = Rc::downgrade(&entry.0);
    entry.0.activate.replace(Some(callback!(
        [menu, weak_entry, props.group, props.on_select] || {
            if let Some(selected) = weak_entry.upgrade() {
                let group = group.get();
                for entry in menu.0.entries.borrow().iter() {
                    if matches!(entry.0.kind, MenuEntryKind::Radio)
                        && entry.0.group.borrow().as_deref() == Some(group.as_str())
                    {
                        entry.0.checked.set(Rc::ptr_eq(&entry.0, &selected));
                    }
                }
                menu.changed();
                if let Some(select) = on_select.get() {
                    select();
                }
            }
        }
    )));
    place_entry(element, menu.clone(), entry.clone());
    common_effects(
        menu.clone(),
        entry.clone(),
        props.label.clone(),
        props.enabled.clone(),
        props.visible.clone(),
        props.shortcut.clone(),
    );
    scoped_effect!(
        [menu, entry, props.selected] || {
            entry.0.checked.set(selected.get());
            menu.changed();
        }
    );
    scoped_effect!(
        [menu, entry, props.group] || {
            *entry.0.group.borrow_mut() = Some(group.get());
            menu.changed();
        }
    );
}

/// Adds a descriptive separator between adjacent menu entries.
#[component]
pub fn MenuSeparator(props: &MenuSeparatorProps, element: &Element) {
    let menu = element.context::<MenuModelContext>().unwrap().0.clone();
    let entry = new_entry(MenuEntryKind::Separator);
    place_entry(element, menu.clone(), entry.clone());
    scoped_effect!(
        [menu, entry, props.visible] || {
            entry.0.visible.set(visible.get());
            menu.changed();
        }
    );
}

/// Properties for a menu presented from a visual element.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ContextMenuProps {
    /// Menu element to present.
    pub menu: Element,
    /// Optional imperative presentation controller.
    #[props(inspect(skip))]
    pub controller: Option<ContextMenuController>,
    /// Visual element that owns the context-menu interaction.
    pub children: Element,
}
