use nestix::{Element, Layout, component, computed, layout};
use nestix_native_core::{StyleContext, StyleScope, WindowProps, dpi::LogicalSize, matched_style};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Node, ResizeObserver};

use crate::{
    dom::{create_html_element, document, mount_host},
    style::{apply_view_style, set},
};

/// DOM application window surface.
#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__dom_Window"];

    let html = create_html_element("div");
    let node = html.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = computed!([matched] || Some(matched.get().unwrap_or_default()));

    element.scoped_effect({
        let html = html.clone();
        let effective_style = effective_style.clone();
        let visible = props.visible.clone();
        move || {
            apply_view_style(&html.style(), &effective_style.get().unwrap_or_default());
            set(
                &html.style(),
                "display",
                if visible.get() { "contents" } else { "none" },
            );
        }
    });
    element.scoped_effect({
        let title = props.title.clone();
        move || document().set_title(&title.get())
    });

    let resize_html = html.clone();
    let on_resize = props.on_resize.clone();
    let resize_listener = Closure::<dyn FnMut(js_sys::Array, ResizeObserver)>::new(move |_, _| {
        if let Some(on_resize) = on_resize.get() {
            on_resize(nestix_native_core::dpi::Size::Logical(LogicalSize::new(
                f64::from(resize_html.client_width()),
                f64::from(resize_html.client_height()),
            )));
        }
    });
    let resize_observer = ResizeObserver::new(resize_listener.as_ref().unchecked_ref())
        .expect("failed to create ResizeObserver");
    resize_observer.observe(&html);
    element.on_unmount(move || {
        resize_observer.disconnect();
        let _ = &resize_listener;
    });

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            $(props.children.clone().map(|child| Layout::from(child.clone())))
        }
    }
}
