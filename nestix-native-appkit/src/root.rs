use nestix::{
    Element, PropValue, State, closure, component, components::ContextProvider, create_state,
    layout, scoped_effect,
};
use nestix_native_core::{RootProps, StyleScope};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSMenu};
use objc2_foundation::{NSObject, NSObjectProtocol};

#[derive(Clone)]
pub struct RootContext {
    pub ns_application: Retained<NSApplication>,
    pub(crate) app_menu: State<Option<Retained<NSMenu>>>,
    pub(crate) active_window_menu: State<Option<Retained<NSMenu>>>,
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__appkit_Root"];

    let mtm = MainThreadMarker::new().unwrap();
    let ns_application = NSApplication::sharedApplication(mtm);
    let app_menu = create_state(None::<Retained<NSMenu>>);
    let active_window_menu = create_state(None::<Retained<NSMenu>>);

    ns_application.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let app_delegate = AppDelegate::new(
        mtm,
        AppState {
            should_terminate_after_last_window_closed: props.quit_when_all_windows_closed.clone(),
        },
    );
    ns_application.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

    element.provide_handle(ns_application.as_ref() as *const NSObject);

    scoped_effect!(
        element,
        [ns_application, app_menu, active_window_menu] || {
            let active_window_menu = active_window_menu.get();
            let app_menu = app_menu.get();
            ns_application.setMainMenu(active_window_menu.as_deref().or(app_menu.as_deref()));
        }
    );

    element.after_mount(closure!(
        [ns_application] || {
            ns_application.run();
        }
    ));

    layout! {
        ContextProvider<RootContext>(
            RootContext {
                ns_application,
                app_menu,
                active_window_menu,
            }
        ) {
            StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                $(props.children.clone())
            }
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
