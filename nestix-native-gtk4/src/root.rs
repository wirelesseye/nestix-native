use std::{cell::Cell, rc::Rc};

use gtk4::glib;
use nestix::{Element, PropValue, closure, component, components::ContextProvider, layout};
use nestix_native_core::{RootProps, StyleScope};

#[derive(Clone)]
pub struct RootContext {
    pub(crate) main_loop: glib::MainLoop,
    pub(crate) window_count: Rc<Cell<usize>>,
    pub(crate) quit_when_all_windows_closed: PropValue<bool>,
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__gtk4_Root"];

    gtk4::init().expect("failed to initialize GTK4");
    let context = RootContext {
        main_loop: glib::MainLoop::new(None, false),
        window_count: Rc::new(Cell::new(0)),
        quit_when_all_windows_closed: props.quit_when_all_windows_closed.clone(),
    };
    element.provide_handle(context.main_loop.clone());

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
