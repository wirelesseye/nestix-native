use nestix::{
    Element, State, closure, component, components::ContextProvider, create_state, layout,
    scoped_effect,
};
use nestix_native_core::{RootProps, StyleScope};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSMenu};
use objc2_foundation::NSObject;

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

    element.provide_handle(ns_application.as_ref() as *const NSObject);
    element.on_unmount(closure!([ns_application] || ns_application.stop(None)));

    scoped_effect!(
        element,
        [ns_application, app_menu, active_window_menu] || {
            let active_window_menu = active_window_menu.get();
            let app_menu = app_menu.get();
            replace_main_menu(
                &ns_application,
                active_window_menu.as_deref().or(app_menu.as_deref()),
            );
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

fn replace_main_menu(application: &NSApplication, menu: Option<&NSMenu>) {
    // Fully detach the previous menu before installing the next one. Reusing a
    // detached NSMenu directly can leave AppKit's key-equivalent lookup in the
    // tracking state produced by the previously focused window (including a
    // failed shortcut lookup).
    if let Some(current) = application.mainMenu() {
        current.cancelTracking();
    }
    application.setMainMenu(None);

    if let Some(menu) = menu {
        menu.update();
        application.setMainMenu(Some(menu));
    }
}
