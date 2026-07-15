#![allow(dead_code, non_snake_case)]

use nestix::{Element, callback, component, layout};
use nestix_native::{
    CheckMenuItem, ContextMenu, FlexView, Menu, MenuBar, MenuItem, MenuSeparator, RadioMenuItem,
    Shortcut, Submenu, Window,
};

#[component]
fn DocumentMenu() -> Element {
    layout! {
        Menu {
            Submenu("File") {
                MenuItem(
                    "Save",
                    .shortcut = Shortcut::primary('S'),
                    .on_activate = callback!(|| {}),
                )
                MenuSeparator()
                CheckMenuItem(
                    "Auto Save",
                    .checked = true,
                    .on_checked_change = callback!(|_value| {}),
                )
                RadioMenuItem(
                    "Plain Text",
                    .group = "format",
                    .selected = true,
                    .on_select = callback!(|| {}),
                )
            }
        }
    }
}

#[component]
fn Target() -> Element {
    layout! { MenuSeparator() }
}

#[test]
fn menu_bar_and_context_menu_compile_through_layout() {
    let _window = layout! {
        Window {
            FlexView {
                MenuBar(.menu = layout! {
                    DocumentMenu
                })
                Target
            }
        }
    };

    let _context = layout! {
        ContextMenu(.menu = layout! { DocumentMenu }) {
            Target
        }
    };
}
