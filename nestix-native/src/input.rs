use nestix::{Element, component};

pub use nestix_native_core::InputProps;

use crate::BackendContext;

#[component]
pub fn Input(props: &InputProps, element: &Element) -> Option<Element> {
    let backend = element.context::<BackendContext>().unwrap().backend;
    let platform_input = backend.create_input(props.clone());

    platform_input
}
