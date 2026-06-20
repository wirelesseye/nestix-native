use nestix::{Element, component};

pub use nestix_native_core::WindowProps;

use crate::BackendContext;

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_window = backend.create_window(props.clone());

    platform_window
}
