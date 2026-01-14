use nestix::{Element, component};

pub use nestix_native_core::{TabViewProps, TabViewItemProps};

use crate::BackendContext;

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_view = backend.create_tab_view(props.clone());

    if let Some(platform_view) = &platform_view {
        element.forward_handle(platform_view);
    }

    platform_view
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_item = backend.create_tab_view_item(props.clone());

    if let Some(platform_item) = &platform_item {
        element.forward_handle(platform_item);
    }

    platform_item
}
