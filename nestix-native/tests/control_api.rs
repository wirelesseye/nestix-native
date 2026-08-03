#![allow(dead_code, non_snake_case)]

use nestix::{Element, callback, component, layout};
use nestix_native::{
    Checkbox, Color, FlexView, RadioButton, Select, SelectOption, Sidebar, Slider, Switch, Window,
};

#[component]
fn FormControls() -> Element {
    layout! {
        FlexView(
            .border(
                .horizontal_width = 1,
                .top_width = 2,
                .color = Some(Color::RED),
                .radius = 8,
            ),
        ) {
            Checkbox("Show details", .checked = true, .on_checked_change = callback!(|_checked| {}))
            RadioButton(
                "Compact",
                .group = "density",
                .selected = true,
                .on_select = callback!(|| {}),
            )
            RadioButton("Comfortable", .group = "density", .on_select = callback!(|| {}))
            Switch(.checked = true, .on_checked_change = callback!(|_checked| {}))
            Select(
                .value = Some("second".to_string()),
                .on_value_change = callback!(|_value: &str| {}),
            ) {
                SelectOption("First", .value = "first")
                SelectOption("Second", .value = "second", .enabled = true)
            }
            Slider(
                .value = 25.0,
                .minimum = 0.0,
                .maximum = 50.0,
                .on_value_change = callback!(|_value| {}),
            )
        }
    }
}

#[test]
fn form_controls_compile_through_layout() {
    let _window = layout! {
        Window(.desktop(.on_close_requested = callback!(|| {}))) {
            FlexView {
                Sidebar(
                    .width = Some(280.0),
                    .min_width = Some(220.0),
                    .resizable = false,
                    .open = Some(true),
                    .on_open_change = callback!(|_open| {}),
                ) {
                    FormControls
                }
                FormControls
            }
        }
    };
}
