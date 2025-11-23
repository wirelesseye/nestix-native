use nestix::{
    Element, closure, component, components::ContextProvider, derive_props, layout, post_update,
    prop::PropValue, provide_handle,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{NSObject, NSObjectProtocol};

#[derive_props]
pub struct AppkitAppProps {
    children: Option<Vec<Element>>,
    
    #[props(default)]
    should_terminate_after_last_window_closed: bool,
}

#[derive(Clone)]
pub struct AppkitAppContext {
    pub app: Retained<NSApplication>,
}

#[component]
pub fn AppkitApp(props: &AppkitAppProps) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);

    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let app_delegate = AppDelegate::new(
        mtm,
        AppState {
            should_terminate_after_last_window_closed: props
                .should_terminate_after_last_window_closed
                .clone(),
        },
    );
    app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

    provide_handle(app.as_ref() as *const NSApplication);

    post_update(closure!(app => || {
        app.run();
    }));

    layout! {
        ContextProvider<AppkitAppContext>(
            .value = AppkitAppContext {
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
