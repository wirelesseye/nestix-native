use nestix::{Element, Layout, component, components::Fragment, layout, props};

use crate::renderer::renderer;

/// Root node of one embedded managed DOM document.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DomDocumentRootProps {
    #[props(default)]
    pub children: Layout,
}

#[component]
pub fn DomDocumentRoot(props: &DomDocumentRootProps, element: &Element) -> Element {
    let renderer = renderer(element);
    element.provide_handle(renderer.root_handle());
    layout! { Fragment(.children = props.children.clone()) }
}
