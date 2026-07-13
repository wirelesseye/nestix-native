use nestix::{Element, component};

pub use nestix_native_core::{ContentFit, ImageSource, ImageViewProps};

use crate::BackendContext;

#[component]
pub fn ImageView(props: &ImageViewProps, element: &Element) -> Option<Element> {
    let backend = element.context::<BackendContext>().unwrap().backend;
    backend.create_image_view(props.clone())
}
