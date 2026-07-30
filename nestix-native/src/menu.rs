pub use nestix_native_core::{
    CheckMenuItem, CheckMenuItemProps, ContextMenuProps, Menu, MenuBarProps, MenuItem,
    MenuItemProps, MenuProps, MenuSeparator, MenuSeparatorProps, RadioMenuItem, RadioMenuItemProps,
    Submenu, SubmenuProps,
};

delegate!(
    /// Installs a [`Menu`] as the containing window's menu bar.
    pub MenuBar(MenuBarProps) => create_menu_bar,
    /// Attaches a menu that can be presented from its child element.
    pub ContextMenu(ContextMenuProps) => create_context_menu,
);
