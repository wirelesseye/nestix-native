pub use nestix_native_core::{
    CheckMenuItemProps, ContextMenuProps, MenuBarProps, MenuItemProps, MenuProps,
    MenuSeparatorProps, RadioMenuItemProps, SubmenuProps,
};

delegate!(
    pub Menu(MenuProps) => create_menu,
    pub MenuBar(MenuBarProps) => create_menu_bar,
    pub Submenu(SubmenuProps) => create_submenu,
    pub MenuItem(MenuItemProps) => create_menu_item,
    pub CheckMenuItem(CheckMenuItemProps) => create_check_menu_item,
    pub RadioMenuItem(RadioMenuItemProps) => create_radio_menu_item,
    pub MenuSeparator(MenuSeparatorProps) => create_menu_separator,
    pub ContextMenu(ContextMenuProps) => create_context_menu,
);
