use nestix::{callback, create_state, layout, unmount_root};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlButtonElement, HtmlElement, HtmlIFrameElement, HtmlInputElement};

use crate::{
    Button, DomAttribute, DomElement, DomElementRef, DomEvent, DomProperty, FlexView, Input, Root,
    Text, WebView, Window, mount_root,
};
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

#[wasm_bindgen_test]
fn web_view_uses_a_reactive_iframe_and_cleans_up() {
    let document = web_sys::window().unwrap().document().unwrap();
    let target = document.create_element("div").unwrap();
    target.set_id("nestix-web-view-test-root");
    document.body().unwrap().append_child(&target).unwrap();

    let url = create_state("https://example.com/first".to_string());
    let app = layout! {
        Root {
            Window {
                WebView(url.clone(), .view(.width = 320, .height = 180))
            }
        }
    };

    mount_root("#nestix-web-view-test-root", &app);
    let iframe = target
        .query_selector("iframe")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlIFrameElement>()
        .unwrap();
    assert_eq!(
        iframe.get_attribute("src").as_deref(),
        Some("https://example.com/first")
    );
    assert_eq!(iframe.style().get_property_value("width").unwrap(), "320px");
    assert_eq!(
        iframe.style().get_property_value("height").unwrap(),
        "180px"
    );

    url.set("https://example.com/second".to_string());
    assert_eq!(
        iframe.get_attribute("src").as_deref(),
        Some("https://example.com/second")
    );

    unmount_root().unwrap();
    assert!(!target.has_child_nodes());
    target.remove();
}

#[wasm_bindgen_test]
fn custom_elements_support_dom_state_events_and_refs() {
    let document = web_sys::window().unwrap().document().unwrap();
    let target = document.create_element("div").unwrap();
    target.set_id("nestix-custom-element-test-root");
    document.body().unwrap().append_child(&target).unwrap();

    let disabled = create_state(false);
    let value = create_state("initial".to_string());
    let clicks = create_state(0);
    let clicks_for_event = clicks.clone();
    let node_ref = DomElementRef::new();
    let app = layout! {
        Root {
            Window {
                DomElement(
                    "sp-button",
                    .class = "internal_action",
                    .dom_class = "external-action",
                    .attributes = nestix::computed!(
                        [disabled]
                            || vec![
                                DomAttribute::string("variant", "accent"),
                                DomAttribute::boolean("disabled", disabled.get()),
                            ]
                    ),
                    .properties = nestix::computed!(
                        [value]
                            || vec![DomProperty::new("value", JsValue::from_str(&value.get())),]
                    ),
                    .events = vec![DomEvent::new("click", move |_| {
                        clicks_for_event.update(|count| count + 1);
                    })],
                    .node_ref = node_ref.clone(),
                ) {
                    Text("Save")
                }
            }
        }
    };

    mount_root("#nestix-custom-element-test-root", &app);
    let custom = target.query_selector("sp-button").unwrap().unwrap();
    assert_eq!(custom.class_name(), "external-action");
    assert_eq!(custom.get_attribute("variant").as_deref(), Some("accent"));
    assert!(!custom.has_attribute("disabled"));
    assert!(node_ref.get().is_some());
    assert_eq!(
        js_sys::Reflect::get(custom.as_ref(), &JsValue::from_str("value"))
            .unwrap()
            .as_string()
            .as_deref(),
        Some("initial")
    );

    disabled.set(true);
    value.set("updated".to_string());
    assert!(custom.has_attribute("disabled"));
    assert_eq!(
        js_sys::Reflect::get(custom.as_ref(), &JsValue::from_str("value"))
            .unwrap()
            .as_string()
            .as_deref(),
        Some("updated")
    );
    custom
        .dispatch_event(&Event::new("click").unwrap())
        .unwrap();
    assert_eq!(clicks.get(), 1);

    unmount_root().unwrap();
    assert!(node_ref.get().is_none());
    assert!(!target.has_child_nodes());
    target.remove();
}
