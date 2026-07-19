#![allow(dead_code, non_snake_case)]

use nestix::{Element, callback, component, layout};
use nestix_native::{
    Checkbox, FlexView, RadioButton, Select, SelectOption, Slider, Switch, Window,
};

#[component]
fn FormControls() -> Element {
    layout! {
        FlexView {
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
        Window(.on_close_requested = callback!(|| {})) {
            FormControls
        }
    };
}
