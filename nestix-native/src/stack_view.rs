use nestix::{Element, component};

pub use nestix_native_core::StackViewProps;

use crate::BackendContext;

#[component]
pub fn StackView(props: &StackViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_view = backend.create_stack_view(props.clone());

    if let Some(platform_view) = &platform_view {
        element.forward_handle(platform_view);
    }

    platform_view
}
