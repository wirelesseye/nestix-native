use nestix::{Element, component};

pub use nestix_native_core::ListViewProps;

use crate::BackendContext;

#[component]
pub fn ListView(props: &ListViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_view = backend.create_list_view(props.clone());

    if let Some(platform_view) = &platform_view {
        element.forward_handle(platform_view);
    }

    platform_view
}
