use nestix::{Element, component, create_state, scoped_effect};
use nestix_native_core::{StyleContext, WebViewProps, dpi::LogicalSize, matched_style};
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest};
use objc2_web_kit::{WKWebView, WKWebViewConfiguration};

use crate::native_control;

/// AppKit web view backed by WebKit.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__appkit_WebView"];

    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().expect("AppKit WebView must be created on the main thread");
    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    let web_view = unsafe {
        WKWebView::initWithFrame_configuration(
            WKWebView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 150.0)),
            &configuration,
        )
    };

    let view: Retained<NSView> = web_view.clone().into_super();
    native_control::mount_with_intrinsic_size(
        element,
        view,
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
        LogicalSize::new(300.0, 150.0),
    );

    scoped_effect!(
        [web_view, props.url] || {
            let value = NSString::from_str(&url.get());
            let url = NSURL::URLWithString(&value)
                .unwrap_or_else(|| panic!("AppKit WebView received an invalid URL: {:?}", value));
            let request = NSURLRequest::requestWithURL(&url);
            unsafe { web_view.loadRequest(&request) }
                .expect("AppKit WebView failed to start navigation");
        }
    );
}
