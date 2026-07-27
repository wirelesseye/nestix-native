use nestix::{Element, closure};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Node};

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

pub(crate) fn mount_host(element: &Element, node: &Node) {
    element.provide_handle(node.clone());
    element.on_place(closure!(
        [node] | placement | {
            if let Some(predecessor) = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<Node>())
                && let Some(parent) = predecessor.parent_node()
            {
                parent
                    .insert_before(&node, predecessor.next_sibling().as_ref())
                    .expect("failed to place DOM node after its predecessor");
            } else if let Some(parent) = placement
                .parent
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<Node>())
            {
                parent
                    .append_child(&node)
                    .expect("failed to append DOM node to its parent");
            }
        }
    ));
    element.on_unmount(closure!(
        [node] || {
            if let Some(parent) = node.parent_node() {
                parent
                    .remove_child(&node)
                    .expect("failed to remove DOM node during unmount");
            }
        }
    ));
}
