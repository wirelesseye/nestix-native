use nestix::{Element, component};

pub use nestix_native_core::ButtonProps;

use crate::BackendContext;

#[component]
pub fn Button(props: &ButtonProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_button = backend.create_button(props.clone());
    
    platform_button
}
