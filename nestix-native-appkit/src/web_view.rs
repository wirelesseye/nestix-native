use std::{
    cell::RefCell,
    path::{Component, Path},
    rc::Rc,
};

use nestix::{
    Element, callback, closure, component, components::Fragment, create_state, layout,
    scoped_effect,
};
use nestix_native_core::{
    JavaScriptEvaluator, StyleContext, WebViewBridge, WebViewBridgeScriptContext,
    WebViewDevToolsError, WebViewPresenter, WebViewProps, WebViewRegistration, WebViewSource,
    dpi::LogicalSize, matched_style, resolved_view_style,
};
#[cfg(any(debug_assertions, feature = "devtools"))]
use objc2::runtime::AnyObject;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject, sel,
};
use objc2_app_kit::NSView;
use objc2_foundation::{
    NSBundle, NSNumber, NSObject, NSObjectNSKeyValueCoding, NSObjectProtocol, NSPoint, NSRect,
    NSSize, NSString, NSURL, NSURLRequest, ns_string,
};
use objc2_web_kit::{
    WKScriptMessage, WKScriptMessageHandler, WKUserContentController, WKUserScript,
    WKUserScriptInjectionTime, WKWebView, WKWebViewConfiguration,
};

use crate::native_control;

/// AppKit web view backed by WebKit.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__appkit_WebView"];

    let bridge = props.bridge.get();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style_props = resolved_view_style(matched, &props.view);

    let mtm = MainThreadMarker::new().expect("AppKit WebView must be created on the main thread");
    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    #[cfg(any(debug_assertions, feature = "devtools"))]
    if props.inspectable.get() {
        enable_developer_extras(&configuration);
    }
    let message_bridge = bridge.as_ref().map(|bridge| {
        let content_controller = unsafe { configuration.userContentController() };
        let handler = ScriptMessageHandler::new(
            mtm,
            ScriptMessageState {
                bridge: bridge.clone(),
            },
        );
        let handler_name = NSString::from_str(bridge.message_channel_name());
        unsafe {
            content_controller
                .addScriptMessageHandler_name(ProtocolObject::from_ref(&*handler), &handler_name);
        }
        let channel_name = bridge
            .message_channel_name()
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        let post_message_expression = format!(
            "message => window.webkit.messageHandlers['{channel_name}'].postMessage(message)"
        );
        if let Some(source) = bridge.initialization_script(WebViewBridgeScriptContext {
            post_message_expression: &post_message_expression,
        }) {
            let source = NSString::from_str(&source);
            let user_script = unsafe {
                WKUserScript::initWithSource_injectionTime_forMainFrameOnly(
                    WKUserScript::alloc(mtm),
                    &source,
                    WKUserScriptInjectionTime::AtDocumentStart,
                    true,
                )
            };
            unsafe {
                content_controller.addUserScript(&user_script);
            }
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
        [web_view, props.inspectable] || {
            // `inspectable` was added in macOS 13.3. Checking the selector keeps
            // WebView usable on older supported macOS versions.
            if web_view.respondsToSelector(sel!(setInspectable:)) {
                unsafe {
                    web_view.setInspectable(inspectable.get());
                }
            }
        }
    );

    let controller_registration = Rc::new(RefCell::new(None::<WebViewRegistration>));
    scoped_effect!(
        [
            web_view,
            configuration,
            props.inspectable,
            props.controller,
            controller_registration
        ] || {
            controller_registration.borrow_mut().take();
            controller_registration
                .borrow_mut()
                .replace(controller.get().bind(WebViewPresenter {
                    open_dev_tools: callback!(
                        [web_view, configuration, inspectable] || {
                            open_dev_tools(&web_view, &configuration, inspectable.get())
                        }
                    ),
                }));
        }
    );
    element.on_unmount(closure!(
        [controller_registration] || {
            controller_registration.borrow_mut().take();
        }
    ));

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

    if let Some(bridge) = &bridge {
        let evaluator_web_view = web_view.clone();
        let evaluator: JavaScriptEvaluator = Rc::new(move |script| {
            let script = NSString::from_str(script);
            unsafe {
                evaluator_web_view.evaluateJavaScript_completionHandler(&script, None);
            }
        });
        bridge.attach(evaluator);
    }

    scoped_effect!(
        [web_view, props.source] || {
            load_source(&web_view, source.get());
        }
    );

    let view: Retained<NSView> = web_view.into_super();
    native_control::mount_with_intrinsic_size(
        element,
        view,
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
        LogicalSize::new(300.0, 150.0),
    );

    if let (Some(bridge), Some((content_controller, handler, handler_name))) =
        (bridge, message_bridge)
    {
        element.on_unmount(move || {
            bridge.detach();
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

fn open_dev_tools(
    web_view: &WKWebView,
    configuration: &WKWebViewConfiguration,
    inspectable: bool,
) -> Result<(), WebViewDevToolsError> {
    if !inspectable {
        return Err(WebViewDevToolsError::NotInspectable);
    }

    #[cfg(any(debug_assertions, feature = "devtools"))]
    unsafe {
        // WKWebView has no public API for opening Web Inspector. This is the
        // same private inspector path used by Wry/Tauri.
        enable_developer_extras(configuration);
        let inspector: Retained<AnyObject> = msg_send![web_view, _inspector];
        let (): () = msg_send![&inspector, show];
        Ok(())
    }

    #[cfg(not(any(debug_assertions, feature = "devtools")))]
    {
        let _ = (web_view, configuration);
        Err(WebViewDevToolsError::Unsupported(
            "opening Web Inspector in a macOS release build requires the `devtools` feature"
                .to_string(),
        ))
    }
}

#[cfg(any(debug_assertions, feature = "devtools"))]
fn enable_developer_extras(configuration: &WKWebViewConfiguration) {
    let preferences = unsafe { configuration.preferences() };
    let enabled = NSNumber::numberWithBool(true);
    unsafe {
        preferences.setValue_forKey(Some(&enabled), ns_string!("developerExtrasEnabled"));
    }
}

fn load_source(web_view: &WKWebView, source: WebViewSource) {
    match source {
        WebViewSource::Url(url) => {
            let value = NSString::from_str(&url);
            let url = NSURL::URLWithString(&value)
                .unwrap_or_else(|| panic!("AppKit WebView received an invalid URL: {:?}", value));
            let request = NSURLRequest::requestWithURL(&url);
            unsafe { web_view.loadRequest(&request) }
                .expect("AppKit WebView failed to start navigation");
        }
        WebViewSource::Html { html, base_url } => {
            let html = NSString::from_str(&html);
            let base_url = base_url.map(|value| {
                let value = NSString::from_str(&value);
                NSURL::URLWithString(&value).unwrap_or_else(|| {
                    panic!("AppKit WebView received an invalid base URL: {:?}", value)
                })
            });
            unsafe {
                web_view.loadHTMLString_baseURL(&html, base_url.as_deref());
            }
        }
        WebViewSource::Resource {
            path,
            development_path,
        } => {
            let (file_url, read_access_url) =
                resolve_document_resource(&path, development_path.as_deref());
            unsafe {
                web_view.loadFileURL_allowingReadAccessToURL(&file_url, &read_access_url);
            }
        }
    }
}

fn resolve_document_resource(
    resource_path: &Path,
    development_path: Option<&Path>,
) -> (Retained<NSURL>, Retained<NSURL>) {
    assert!(
        !resource_path.as_os_str().is_empty()
            && !resource_path.is_absolute()
            && resource_path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "WebView resource paths must be non-empty relative paths without `..`: {:?}",
        resource_path
    );

    if let Some(resource_root) = NSBundle::mainBundle().resourceURL() {
        let component = NSString::from_str(&resource_path.to_string_lossy());
        if let Some(file_url) = resource_root.URLByAppendingPathComponent(&component)
            && ns_url_is_file(&file_url)
        {
            let read_access_url = file_url
                .URLByDeletingLastPathComponent()
                .expect("bundled DOM template must have a parent directory");
            return (file_url, read_access_url);
        }
    }

    if let Some(development_path) = development_path {
        let canonical = development_path.canonicalize().unwrap_or_else(|error| {
            panic!(
                "failed to resolve development DOM template {:?}: {error}",
                development_path
            )
        });
        assert!(
            canonical.is_file(),
            "development DOM template is not a file: {:?}",
            canonical
        );
        let parent = canonical
            .parent()
            .expect("development DOM template must have a parent directory");
        let file_path = NSString::from_str(&canonical.to_string_lossy());
        let root_path = NSString::from_str(&parent.to_string_lossy());
        return (
            NSURL::fileURLWithPath(&file_path),
            NSURL::fileURLWithPath(&root_path),
        );
    }

    panic!(
        "DOM template resource {:?} was not found in the application bundle; provide a development path when running without a packaged app",
        resource_path
    );
}

fn ns_url_is_file(url: &NSURL) -> bool {
    url.path()
        .map(|path| Path::new(&path.to_string()).is_file())
        .unwrap_or(false)
}

struct ScriptMessageState {
    bridge: Rc<dyn WebViewBridge>,
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
            self.ivars().bridge.receive_message(&body.to_string());
        }
    }
);

impl ScriptMessageHandler {
    fn new(mtm: MainThreadMarker, state: ScriptMessageState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
