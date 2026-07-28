#![allow(dead_code, non_snake_case)]

use nestix::{Element, component, computed, create_state, layout};
use nestix_native::{WebView, WebViewController, WebViewDevToolsError, WebViewSource, Window};

#[component]
fn Browser() -> Element {
    let url = create_state("https://example.com".to_string());
    let controller = WebViewController::new();
    layout! {
        WebView(
            computed!([url] || WebViewSource::url(url.get())),
            .view(.width = 300, .height = 150),
            .inspectable = true,
            .controller = controller,
        )
    }
}

#[test]
fn web_view_compiles_through_layout() {
    let _url_window = layout! {
        Window {
            Browser
        }
    };
    let _html_window = layout! {
        Window {
            WebView(WebViewSource::html("<!doctype html><body>Inline application HTML</body>"))
        }
    };
    let _resource_window = layout! {
        Window {
            WebView(
                WebViewSource::resource("web/index.html")
                    .with_development_path("assets/web/index.html"),
            )
        }
    };
    let controller = WebViewController::new();
    assert_eq!(
        controller.open_dev_tools(),
        Err(WebViewDevToolsError::NotMounted)
    );
}
