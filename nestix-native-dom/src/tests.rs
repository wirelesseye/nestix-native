use nestix::{callback, create_state, layout, unmount_root};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlButtonElement, HtmlElement, HtmlInputElement};

use crate::{Button, FlexView, Input, Root, Text, Window, mount_root};
use nestix_native_core::{StyleProvider, style};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn mounts_reacts_and_cleans_up() {
    let document = web_sys::window().unwrap().document().unwrap();
    let target = document.create_element("div").unwrap();
    target.set_id("nestix-dom-test-root");
    document.body().unwrap().append_child(&target).unwrap();

    let count = create_state(0);
    let input_value = create_state(String::new());
    let styles = style! {
        .rust_only_class {
            gap: 7 px;
            bg_color: #112233;

            > .__Text {
                margin_top: 3 px;
            }
        }
    };
    let app = layout! {
        StyleProvider(styles) {
            Root {
                Window(.title = "DOM test", .width = 320, .height = 200) {
                    FlexView(.class = "rust_only_class") {
                        Text(nestix::computed!([count] || format!("Count: {}", count.get())))
                        Button(
                            .title = "Increment",
                            .on_click = callback!([count] || count.update(|value| value + 1)),
                        )
                        Input(
                            .value = input_value.clone(),
                            .on_text_change = callback!(
                                [input_value] |value: &str| {
                                    input_value.set(value.to_string());
                                }
                            ),
                        )
                    }
                }
            }
        }
    };

    mount_root("#nestix-dom-test-root", &app);
    assert_eq!(document.title(), "DOM test");
    assert!(target.query_selector(".rust_only_class").unwrap().is_none());
    let flex = target
        .query_selector("div > div")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    assert_eq!(flex.style().get_property_value("gap").unwrap(), "7px");
    assert_eq!(
        target
            .query_selector("span")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .get_property_value("margin-top")
            .unwrap(),
        "3px"
    );

    let button = target
        .query_selector("button")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlButtonElement>()
        .unwrap();
    button.click();
    assert_eq!(
        target
            .query_selector("span")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Count: 1")
    );

    let input = target
        .query_selector("input")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    input.set_value("hello");
    input.dispatch_event(&Event::new("input").unwrap()).unwrap();
    assert_eq!(input_value.get(), "hello");

    unmount_root().unwrap();
    assert!(!target.has_child_nodes());
    target.remove();
}

#[wasm_bindgen_test]
#[should_panic(expected = "did not match an element")]
fn missing_mount_selector_panics() {
    let app = layout! { Root() };
    mount_root("#nestix-dom-missing-target", &app);
}
