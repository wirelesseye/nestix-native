#![allow(dead_code, non_snake_case)]

use nestix::{Element, component, create_state, layout};
use nestix_native::{WebView, Window};

#[component]
fn Browser() -> Element {
    let url = create_state("https://example.com".to_string());
    layout! {
        WebView(url.clone(), .view(.width = 300, .height = 150))
    }
}

#[test]
fn web_view_compiles_through_layout() {
    let _window = layout! {
        Window {
            Browser
        }
    };
}
