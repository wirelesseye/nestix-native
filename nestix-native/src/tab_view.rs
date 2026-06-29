use nestix::{Element, component};

pub use nestix_native_core::{TabViewItemProps, TabViewProps};

use crate::BackendContext;

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_view = backend.create_tab_view(props.clone());

    platform_view
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Option<Element> {
    let backend = &element.context::<BackendContext>().unwrap().backend;
    let platform_item = backend.create_tab_view_item(props.clone());

    platform_item
}
