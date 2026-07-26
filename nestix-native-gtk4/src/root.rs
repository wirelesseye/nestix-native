use gtk4::glib;
use nestix::{Element, closure, component, components::ContextProvider, layout};
use nestix_native_core::{DEFAULT_ROOT_FONT_SIZE, RootProps, StyleScope};

#[derive(Clone)]
pub struct RootContext {
    pub(crate) main_loop: glib::MainLoop,
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Root", "__gtk4_Root"];

    gtk4::init().expect("failed to initialize GTK4");
    let initial_font_size = gtk4::Settings::default()
        .and_then(|settings| settings.gtk_font_name())
        .map(|name| gtk4::pango::FontDescription::from_string(&name))
        .map(|description| {
            let size = f64::from(description.size()) / f64::from(gtk4::pango::SCALE);
            if description.is_size_absolute() {
                size
            } else {
                size * 96.0 / 72.0
            }
        })
        .filter(|size| size.is_finite() && *size > 0.0)
        .unwrap_or(DEFAULT_ROOT_FONT_SIZE);
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
            StyleScope(
                .class = props.class.clone(),
                .default_classes = DEFAULT_CLASSES,
                .initial_font_size = Some(initial_font_size),
            ) {
                $(props.children.clone())
            }
        }
    }
}
