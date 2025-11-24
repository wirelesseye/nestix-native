use nestix::{Element, component};

pub use nestix_native_core::LabelProps;

use crate::BackendContext;

#[component]
pub fn Label(props: &LabelProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_label = backend.create_label(props.clone());

    if let Some(platform_label) = &platform_label {
        element.forward_handle(platform_label);
    }

    platform_label
}
