use nestix::{Element, callback, component, computed, create_state, layout, unmount_root};
use nestix_native::{
    AlignItems, Button, DomSurface, FlexDirection, FlexView, Input, Root, StyleProvider, Text,
    Window, style,
};

#[component]
pub fn App() -> Element {
    let count = create_state(0);
    let name = create_state("Nestix".to_string());
    let styles = style! {
        .app {
            padding: 20 px;
            gap: 16 px;
        }

        .native_panel, .dom_panel {
            padding: 16 px;
            gap: 10 px;
        }

        .native_actions, .dom_actions {
            flex_direction: row;
            align_items: center;
            gap: 8 px;
        }

        .heading {
            font_size: 20 px;
            font_weight: semi-bold;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix DomSurface",
                    .desktop(
                        .width = 620,
                        .height = 560,
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        })
                    ),
                ) {
                    FlexView(
                        .class = "app",
                        .view(.flex_grow = 1.0),
                        .align_items = AlignItems::Stretch,
                    ) {
                        // These controls are rendered by the native backend.
                        FlexView(.class = "native_panel") {
                            Text("Native controls", .class = "heading")
                            Text(computed!([count] || format!("Shared count: {}", count.get())))
                            Input(
                                .value = name.clone(),
                                .on_text_change = callback!(
                                    [name] |value: &str| {
                                        name.set(value.to_string());
                                    }
                                ),
                            )
                            FlexView(
                                .class = "native_actions",
                                .flex_direction = FlexDirection::Row,
                            ) {
                                Button(
                                    .title = "Increment natively",
                                    .on_click = callback!(
                                        [count] || {
                                            count.update(|value| value + 1)
                                        }
                                    ),
                                )
                                Button(
                                    .title = "Reset",
                                    .on_click = callback!([count] || count.set(0)),
                                )
                            }
                        }
                        // The same Nestix Native components become DOM elements:
                        // <div>, <span>, <input>, and <button> inside WKWebView.
                        DomSurface(
                            .class = "dom_surface",
                            .view(.height = 260, .align_self = AlignItems::Stretch),
                        ) {
                            FlexView(.class = "dom_panel") {
                                Text("DOM elements in DomSurface", .class = "heading")
                                Text(computed!([name] || format!("Hello, {}", name.get())))
                                Text(computed!([count] || format!("Shared count: {}", count.get())))
                                Input(
                                    .value = name.clone(),
                                    .on_text_change = callback!(
                                        [name] |value: &str| {
                                            name.set(value.to_string());
                                        }
                                    ),
                                )
                                FlexView(
                                    .class = "dom_actions",
                                    .flex_direction = FlexDirection::Row,
                                ) {
                                    Button(
                                        .title = "Add ten in the DOM",
                                        .on_click = callback!(
                                            [count] || { count.update(|value| value + 10) }
                                        ),
                                    )
                                    Button(
                                        .title = "Reset",
                                        .on_click = callback!([count] || count.set(0)),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
