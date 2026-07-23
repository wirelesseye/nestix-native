pub use nestix_native_core::{
    CheckMenuItemProps, ContextMenuProps, MenuBarProps, MenuItemProps, MenuProps,
    MenuSeparatorProps, RadioMenuItemProps, SubmenuProps,
};

delegate!(
    /// Groups menu items and submenus into a native menu.
    pub Menu(MenuProps) => create_menu,
    /// Installs a [`Menu`] as the containing window's menu bar.
    pub MenuBar(MenuBarProps) => create_menu_bar,
    /// Displays a labelled menu item that opens a nested menu.
    pub Submenu(SubmenuProps) => create_submenu,
    /// Displays an actionable command in a menu.
    pub MenuItem(MenuItemProps) => create_menu_item,
    /// Displays a menu command with a checked or unchecked state.
    pub CheckMenuItem(CheckMenuItemProps) => create_check_menu_item,
    /// Displays a mutually exclusive choice within a menu group.
    pub RadioMenuItem(RadioMenuItemProps) => create_radio_menu_item,
    /// Draws a separator between adjacent menu items.
    pub MenuSeparator(MenuSeparatorProps) => create_menu_separator,
    /// Attaches a menu that can be presented from its child element.
    pub ContextMenu(ContextMenuProps) => create_context_menu,
);
