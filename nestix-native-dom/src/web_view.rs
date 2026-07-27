use nestix::{Element, component};
use nestix_native_core::{
    StyleContext, WebViewProps, WebViewSource, matched_style, resolved_view_style,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlIFrameElement, Node};

use crate::{
    dom::{create_html_element, mount_host},
    style::apply_view_style,
};

/// DOM web view rendered as an HTML iframe.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__dom_WebView"];

    let iframe = create_html_element("iframe")
        .dyn_into::<HtmlIFrameElement>()
        .expect("iframe element must be an HtmlIFrameElement");
    let node = iframe.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let iframe = iframe.clone();
        let effective_style = effective_style.clone();
        let source = props.source.clone();
        move || {
            match source.get() {
                WebViewSource::Url(url) => {
                    iframe.remove_attribute("srcdoc").unwrap();
                    iframe.set_src(&url);
                }
                WebViewSource::Html { html, base_url } => {
                    iframe.remove_attribute("src").unwrap();
                    let html = match base_url {
                        Some(base_url) => {
                            format!("<base href=\"{}\">{html}", escape_html_attribute(&base_url))
                        }
                        None => html,
                    };
                    iframe.set_srcdoc(&html);
                }
                WebViewSource::Resource { path, .. } => {
                    iframe.remove_attribute("srcdoc").unwrap();
                    iframe.set_src(&path.to_string_lossy());
                }
            }
            apply_view_style(&iframe.style(), &effective_style.get().unwrap_or_default());
        }
    });
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
