use nestix::{Element, PropValue, closure, component, components::ContextProvider, layout};
use nestix_native_core::RootProps;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{NSObject, NSObjectProtocol};

#[derive(Clone)]
pub struct RootContext {
    pub ns_application: Retained<NSApplication>,
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let ns_application = NSApplication::sharedApplication(mtm);

    ns_application.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let app_delegate = AppDelegate::new(
        mtm,
        AppState {
            should_terminate_after_last_window_closed: props.quit_when_all_windows_closed.clone(),
        },
    );
    ns_application.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

    element.provide_handle(ns_application.as_ref() as *const NSObject);

    element.after_render(closure!(
        [ns_application] || {
            ns_application.run();
        }
    ));

    layout! {
        ContextProvider<RootContext>(
            .value = RootContext {
                ns_application,
            }
        ) {
            $(props.children.clone())
        }
    }
}

struct AppState {
    should_terminate_after_last_window_closed: PropValue<bool>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppDelegate"]
    #[ivars = AppState]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn application_should_terminate_after_last_window_closed(&self, _: &NSApplication) -> bool {
            self.ivars().should_terminate_after_last_window_closed.get()
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker, state: AppState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
