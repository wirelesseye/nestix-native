#![allow(dead_code, non_snake_case)]

use nestix::{Element, callback, component, layout};
use nestix_native::{
    CheckMenuItem, ImageSource, Menu, MenuItem, Root, TrayIcon, TrayIconError, TrayIconEvent,
};

#[component]
fn TrayMenu() -> Element {
    layout! {
        Menu {
            MenuItem("Open", .on_activate = callback!(|| {}))
            CheckMenuItem("Enabled", .checked = true)
        }
    }
}

#[test]
fn tray_icon_compiles_through_layout() {
    let visible = nestix::create_state(true);
    let _root = layout! {
        Root {
            TrayIcon(
                .icon = ImageSource::bytes(&[][..]),
                .tooltip = Some("Tray API test".to_string()),
                .visible = visible,
                .menu = Some(layout! { TrayMenu }),
                .on_activate = callback!(|event: TrayIconEvent| {
                    let _: Result<(), TrayIconError> = event.show_menu();
                }),
                .on_secondary = callback!(|event: TrayIconEvent| {
                    let _ = event.show_menu();
                }),
            )
        }
    };
}
