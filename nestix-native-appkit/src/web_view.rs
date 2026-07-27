use std::rc::Rc;

use nestix::{Element, component, components::Fragment, create_state, layout, scoped_effect};
use nestix_native_core::{
    JavaScriptEvaluator, StyleContext, WebViewDocument, WebViewProps, dpi::LogicalSize,
    matched_style, resolved_view_style,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::NSView;
use objc2_foundation::{
    NSNumber, NSObject, NSObjectNSKeyValueCoding, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString, NSURL, NSURLRequest, ns_string,
};
use objc2_web_kit::{
    WKScriptMessage, WKScriptMessageHandler, WKUserContentController, WKWebView,
    WKWebViewConfiguration,
};

use crate::native_control;

/// AppKit web view backed by WebKit.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__appkit_WebView"];

    let document = props.document.get();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style_props = resolved_view_style(matched, &props.view);

    let mtm = MainThreadMarker::new().expect("AppKit WebView must be created on the main thread");
    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    let message_bridge = document.as_ref().map(|document| {
        let content_controller = unsafe { configuration.userContentController() };
        let handler = ScriptMessageHandler::new(
            mtm,
            ScriptMessageState {
                document: document.clone(),
            },
        );
        let handler_name = NSString::from_str(document.message_handler_name());
        unsafe {
            content_controller
                .addScriptMessageHandler_name(ProtocolObject::from_ref(&*handler), &handler_name);
        }
        (content_controller, handler, handler_name)
    });
    let web_view = unsafe {
        WKWebView::initWithFrame_configuration(
            WKWebView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 150.0)),
            &configuration,
        )
    };

    scoped_effect!(
        [web_view, props.transparent] || {
            // WKWebView paints an opaque white backing even when the document is
            // transparent. `drawsBackground` is available through KVC on macOS.
            let draws_background = NSNumber::numberWithBool(!transparent.get());
            unsafe {
                web_view.setValue_forKey(Some(&draws_background), ns_string!("drawsBackground"));
            }
        }
    );

    if let Some(document) = &document {
        let evaluator_web_view = web_view.clone();
        let evaluator: JavaScriptEvaluator = Rc::new(move |script| {
            let script = NSString::from_str(script);
            unsafe {
                evaluator_web_view.evaluateJavaScript_completionHandler(&script, None);
            }
        });
        document.attach(evaluator);
        let html = NSString::from_str(document.html());
        unsafe {
            web_view.loadHTMLString_baseURL(&html, None);
        }
    } else {
        scoped_effect!(
            [web_view, props.url] || {
                let value = NSString::from_str(&url.get());
                let url = NSURL::URLWithString(&value).unwrap_or_else(|| {
                    panic!("AppKit WebView received an invalid URL: {:?}", value)
                });
                let request = NSURLRequest::requestWithURL(&url);
                unsafe { web_view.loadRequest(&request) }
                    .expect("AppKit WebView failed to start navigation");
            }
        );
    }

    let view: Retained<NSView> = web_view.into_super();
    native_control::mount_with_intrinsic_size(
        element,
        view,
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
        LogicalSize::new(300.0, 150.0),
    );

    if let (Some(document), Some((content_controller, handler, handler_name))) =
        (document, message_bridge)
    {
        element.on_unmount(move || {
            document.detach();
            unsafe {
                content_controller.removeScriptMessageHandlerForName(&handler_name);
            }
            let _ = &handler;
        });
    }

    layout! {
        Fragment {
            $(props.children.get())
        }
    }
}

struct ScriptMessageState {
    document: Rc<dyn WebViewDocument>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixWebViewScriptMessageHandler"]
    #[ivars = ScriptMessageState]
    struct ScriptMessageHandler;

    unsafe impl NSObjectProtocol for ScriptMessageHandler {}

    unsafe impl WKScriptMessageHandler for ScriptMessageHandler {
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn user_content_controller_did_receive_script_message(
            &self,
            _: &WKUserContentController,
            message: &WKScriptMessage,
        ) {
            let body = unsafe { message.body() };
            let Some(body) = body.downcast_ref::<NSString>() else {
                return;
            };
            self.ivars().document.receive_message(&body.to_string());
        }
    }
);

impl ScriptMessageHandler {
    fn new(mtm: MainThreadMarker, state: ScriptMessageState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
