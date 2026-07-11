use nestix::{Element, component};

pub use nestix_native_core::ScrollViewProps;

use crate::BackendContext;

#[component]
pub fn ScrollView(props: &ScrollViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    backend.create_scroll_view(props.clone())
}
