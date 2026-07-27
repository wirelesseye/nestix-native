use nestix::{Element, callback, component, create_state, layout, unmount_root};
use nestix_native::{
    AlignItems, Button, FlexDirection, FlexView, Input, Root, StyleProvider, Text, WebView, Window,
    style,
};

#[component]
pub fn App() -> Element {
    let address = create_state("https://example.com".to_string());
    let loaded_url = create_state(address.get());
    let styles = style! {
        .content {
            padding: 16 px;
            gap: 12 px;
        }

        .toolbar {
            flex_direction: row;
            align_items: center;
            gap: 8 px;
        }

        .address {
            flex_grow: 1;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix WebView",
                    .width = 900,
                    .height = 650,
                    .desktop(.on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    })),
                ) {
                    FlexView(
                        .class = "content",
                        .view(.flex_grow = 1.0),
                        .align_items = AlignItems::Stretch,
                    ) {
                        Text("Enter a URL and select Go to navigate the WebView.")
                        FlexView(
                            .class = "toolbar",
                            .flex_direction = FlexDirection::Row,
                            .align_items = AlignItems::Center,
                        ) {
                            Input(
                                .class = "address",
                                .value = address.clone(),
                                .on_text_change = callback!(
                                    [address] |value: &str| {
                                        address.set(value.to_string());
                                    }
                                ),
                            )
                            Button(
                                .title = "Go",
                                .on_click = callback!(
                                    [address, loaded_url] || {
                                        loaded_url.set(address.get().trim().to_string());
                                    }
                                ),
                            )
                        }
                        WebView(
                            loaded_url.clone(),
                            .view(.flex_grow = 1.0, .align_self = AlignItems::Stretch),
                        )
                    }
                }
            }
        }
    }
}
