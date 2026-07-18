use gtk4::glib;
use nestix::{Element, closure, component, components::ContextProvider, layout};
use nestix_native_core::{RootProps, StyleScope};

#[derive(Clone)]
pub struct RootContext {
    pub(crate) main_loop: glib::MainLoop,
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__gtk4_Root"];

    gtk4::init().expect("failed to initialize GTK4");
    let main_loop = glib::MainLoop::new(None, false);
    let context = RootContext { main_loop };
    element.provide_handle(context.main_loop.clone());
    element.on_unmount(closure!([context] || context.main_loop.quit()));

    element.after_mount(closure!(
        [context] || {
            context.main_loop.run();
        }
    ));

    layout! {
        ContextProvider<RootContext>(context) {
            StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                $(props.children.clone())
            }
        }
    }
}
