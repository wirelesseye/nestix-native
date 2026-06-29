use nestix::{Element, component};

pub use nestix_native_core::TextProps;

use crate::BackendContext;

#[component]
pub fn Text(props: &TextProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_text = backend.create_text(props.clone());

    platform_text
}
