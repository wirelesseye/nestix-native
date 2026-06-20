use nestix::{Element, component};

pub use nestix_native_core::FlexViewProps;

use crate::BackendContext;

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_view = backend.create_flex_view(props.clone());

    platform_view
}
