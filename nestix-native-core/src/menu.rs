use std::{
    fmt,
    ops::{BitOr, BitOrAssign},
};

use nestix::{Element, Layout, Shared, props};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuProps {
    #[props(default)]
    pub children: Layout,
}

/// Presents a [`Menu`] as a window menu bar at this position in the visual
/// tree.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuBarProps {
    pub menu: Element,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct SubmenuProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default = true)]
    pub visible: bool,
    #[props(default)]
    pub children: Layout,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct MenuItemProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default = true)]
    pub visible: bool,
    pub shortcut: Option<Shortcut>,
    pub on_activate: Option<Shared<dyn Fn()>>,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct CheckMenuItemProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default = true)]
    pub visible: bool,
    pub shortcut: Option<Shortcut>,
    #[props(default)]
    pub checked: bool,
    pub on_checked_change: Option<Shared<dyn Fn(bool)>>,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct RadioMenuItemProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default = true)]
    pub visible: bool,
    pub shortcut: Option<Shortcut>,
    #[props(default)]
    pub selected: bool,
    pub group: String,
    pub on_select: Option<Shared<dyn Fn()>>,
}

#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct MenuSeparatorProps {
    #[props(default = true)]
    pub visible: bool,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ContextMenuProps {
    pub menu: Element,
    pub children: Element,
}
