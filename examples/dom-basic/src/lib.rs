#![cfg(target_arch = "wasm32")]

use std::mem;

use nestix::{Element, callback, component, computed, create_state, layout};
use nestix_native::dom::{DomAttribute, DomElement, DomEvent};
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
    let (count, set_count) = create_state(0);
    let (name, set_name) = create_state(String::new());
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
                Window(.title = "Nestix Native DOM") {
                    FlexView(.class = "app") {
                        Text("Nestix Native DOM", .class = "heading")
                        Text(computed!([count] || format!("Count: {}", count.get())))
                        Input(
                            .value = name.clone(),
                            .on_text_change = callback!(
                                [name] |value: &str| {
                                    set_name.set(value.to_string());
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
                                .on_click = callback!(
                                    [count] || set_count.update(|value| value + 1)
                                ),
                            )
                            Button(
                                .title = "Reset",
                                .on_click = callback!([count] || set_count.set(0)),
                            )
                            DomElement(
                                "demo-button",
                                .attributes = vec![DomAttribute::string("variant", "accent")],
                                .events = vec![DomEvent::new("click", {
                                    let count = count.clone();
                                    move |_| set_count.update(|value| value + 10)
                                })],
                            ) {
                                Text("Add ten (custom element)")
                            }
                        }
                    }
                }
            }
        }
    }
}
