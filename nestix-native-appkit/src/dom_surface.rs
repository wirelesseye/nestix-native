use std::sync::atomic::{AtomicU64, Ordering};

use nestix::{Element, component, components::ContextProvider, create_state, layout};
use nestix_native_core::{
    DomSurfaceProps, StyleContext, dpi::LogicalSize, matched_style, resolved_view_style,
};
use nestix_native_dom::{DOM_BOOTSTRAP_HTML, DomRuntimeContext, DomSurfaceId, EmbeddedDomRuntime};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::NSView;
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::{
    WKScriptMessage, WKScriptMessageHandler, WKUserContentController, WKWebView,
    WKWebViewConfiguration,
};

use crate::native_control;

static NEXT_SURFACE_ID: AtomicU64 = AtomicU64::new(1);

#[component]
pub fn DomSurface(props: &DomSurfaceProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__DomSurface", "__appkit_DomSurface"];

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style_props = resolved_view_style(matched, &props.view);
    let runtime = EmbeddedDomRuntime::new(DomSurfaceId(
        NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed),
    ));

    let mtm = MainThreadMarker::new().expect("AppKit DomSurface must run on the main thread");
    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    let content_controller = unsafe { configuration.userContentController() };
    let handler = ScriptMessageHandler::new(
        mtm,
        ScriptMessageState {
            runtime: runtime.clone(),
        },
    );
    let handler_name = NSString::from_str("nestix");
    unsafe {
        content_controller
            .addScriptMessageHandler_name(ProtocolObject::from_ref(&*handler), &handler_name);
    }

    let web_view = unsafe {
        WKWebView::initWithFrame_configuration(
            WKWebView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 150.0)),
            &configuration,
        )
    };
    let sender_web_view = web_view.clone();
    runtime.set_sender(move |commands| {
        let script = NSString::from_str(&format!("window.__nestixApply({commands});"));
        unsafe {
            sender_web_view.evaluateJavaScript_completionHandler(&script, None);
        }
    });

    let view: Retained<NSView> = web_view.clone().into_super();
    native_control::mount_with_intrinsic_size(
        element,
        view,
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
        LogicalSize::new(300.0, 150.0),
    );

    let html = NSString::from_str(DOM_BOOTSTRAP_HTML);
    unsafe {
        web_view.loadHTMLString_baseURL(&html, None);
    }

    element.on_unmount({
        let runtime = runtime.clone();
        move || {
            runtime.clear_sender();
            unsafe {
                content_controller.removeScriptMessageHandlerForName(&handler_name);
            }
            let _ = &handler;
        }
    });

    layout! {
        ContextProvider<DomRuntimeContext>(DomRuntimeContext { runtime }) {
            $(props.children.clone())
        }
    }
}

struct ScriptMessageState {
    runtime: std::rc::Rc<EmbeddedDomRuntime>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixDomScriptMessageHandler"]
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
            if let Err(error) = self.ivars().runtime.handle_message_json(&body.to_string()) {
                eprintln!("ignored invalid Nestix DOM message: {error}");
            }
        }
    }
);

impl ScriptMessageHandler {
    fn new(mtm: MainThreadMarker, state: ScriptMessageState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
