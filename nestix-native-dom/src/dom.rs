use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

pub(crate) fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("nestix-native-dom requires a browser document")
}

pub(crate) fn create_html_element(tag: &str) -> HtmlElement {
    document()
        .create_element(tag)
        .unwrap_or_else(|_| panic!("failed to create DOM element `{tag}`"))
        .dyn_into()
        .unwrap_or_else(|_| panic!("DOM element `{tag}` is not an HtmlElement"))
}
