use std::rc::Rc;

use nestix::{
    Element, callback, closure, component, components::ContextProvider, create_state, layout,
    scoped_effect,
};
use nestix_native_core::{StyleScope, TreeContext, WindowProps};

use crate::{
    contexts::{AppContext, ParentContext},
    xaml::XamlElement,
};

#[derive(Clone)]
pub struct WindowContext {
    pub scale_factor: nestix::Readonly<f64>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__winui_Window"];

    let app_context = element.context::<AppContext>().unwrap();
    let scale_factor = create_state(1.0);
    let window_context = Rc::new(WindowContext {
        scale_factor: scale_factor.clone().into_readonly(),
    });
    let tree_context = Rc::new(TreeContext::new());

    let window = XamlElement::window(props.title.get(), props.width.get(), props.height.get())
        .expect("failed to create WinUI window");
    element.provide_handle(window.clone());

    element.after_mount(closure!(
        [window] || {
            let _ = window.activate();
        }
    ));

    scoped_effect!(
        element,
        [window, props.title] || {
            let _ = window.set_text(title.get());
        }
    );

    scoped_effect!(
        element,
        [window, props.width, props.height] || {
            let _ = window.set_size(width.get(), height.get());
        }
    );

    element.on_unmount(closure!(
        [app_context] || {
            app_context.app.quit();
        }
    ));

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context) {
                StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                    ContextProvider<ParentContext>(
                        ParentContext {
                            add_child: callback!([window] |child: XamlElement| {
                                let _ = window.append_child(child);
                            }),
                            remove_child: callback!([window] |child: &XamlElement| {
                                let _ = window.remove_child(child);
                            }),
                        },
                    ) {
                        $(props.children.get())
                    }
                }
            }
        }
    }
}
