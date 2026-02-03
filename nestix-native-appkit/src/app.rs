use nestix::{Element, closure, component, components::ContextProvider, layout, PropValue};
use nestix_native_core::AppProps;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{NSObject, NSObjectProtocol};

#[derive(Clone)]
pub struct AppContext {
    pub app: Retained<NSApplication>,
}

#[component]
pub fn App(props: &AppProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);

    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let app_delegate = AppDelegate::new(
        mtm,
        AppState {
            should_terminate_after_last_window_closed: props.quit_when_all_windows_closed.clone(),
        },
    );
    app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

    element.provide_handle(app.as_ref() as *const NSObject);

    element.after_render(closure!(
        [app] || {
            app.run();
        }
    ));

    layout! {
        ContextProvider<AppContext>(
            .value = AppContext {
                app
            },
            .children = props.children.clone(),
        )
    }
}

struct AppState {
    should_terminate_after_last_window_closed: PropValue<bool>,
}

define_class! {
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
}

impl AppDelegate {
    fn new(mtm: MainThreadMarker, state: AppState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
