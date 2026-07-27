#![cfg(target_arch = "wasm32")]

use std::mem;

use nestix::{Element, callback, component, computed, create_state, layout};
use nestix_native::{
    AlignItems, Button, FlexDirection, FlexView, Input, Root, StyleProvider, Text, Window, style,
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn start() {
    let app = layout! { App };
    nestix_native::dom::mount_root("#app", &app);
    mem::forget(app);
}

#[component]
fn App() -> Element {
    let count = create_state(0);
    let name = create_state(String::new());
    let styles = style! {
        .app {
            padding: 24 px;
            gap: 16 px;
            bg_color: #F4F6F8;
        }

        .actions {
            flex_direction: row;
            align_items: center;
            gap: 8 px;
        }

        .heading {
            font_size: 24 px;
            font_weight: semi-bold;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(.title = "Nestix Native DOM", .width = 480, .height = 320) {
                    FlexView(.class = "app") {
                        Text("Nestix Native DOM", .class = "heading")
                        Text(computed!([count] || format!("Count: {}", count.get())))
                        Input(
                            .value = name.clone(),
                            .on_text_change = callback!(
                                [name] |value: &str| {
                                    name.set(value.to_string());
                                }
                            ),
                        )
                        Text(computed!([name] || format!("Hello, {}", name.get())))
                        FlexView(
                            .class = "actions",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Button(
                                .title = "Increment",
                                .on_click = callback!([count] || count.update(|value| value + 1)),
                            )
                            Button(.title = "Reset", .on_click = callback!([count] || count.set(0)))
                        }
                    }
                }
            }
        }
    }
}
