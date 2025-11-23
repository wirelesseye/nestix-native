use nestix::{
    Element, closure, component, components::ContextProvider, derive_props, layout, post_update, provide_handle
};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

#[derive_props]
pub struct AppkitAppProps {
    children: Option<Vec<Element>>,
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

    // let app_delegate = AppDelegate::new(mtm, app_model.clone());
    // app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

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
