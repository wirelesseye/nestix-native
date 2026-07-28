use std::rc::Rc;

use nestix::{Element, Shared};

use crate::{DomEventData, DomEventOptions, DomNodeHandle, DomStyle, DomValue, EmbeddedDomRuntime};

pub(crate) type DomEventListener = Shared<dyn Fn(&DomEventData)>;

pub(crate) trait DomRenderer {
    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn root_handle(&self) -> DomNodeHandle;
    fn create_element(&self, tag: &str) -> DomNodeHandle;
    fn set_text(&self, node: DomNodeHandle, value: String);
    fn replace_styles(&self, node: DomNodeHandle, styles: Vec<DomStyle>);
    fn set_attribute(&self, node: DomNodeHandle, name: String, value: Option<String>);
    fn set_property(&self, node: DomNodeHandle, name: String, value: DomValue);
    fn remove_property(&self, node: DomNodeHandle, name: String);
    fn place(&self, node: DomNodeHandle, parent: DomNodeHandle, predecessor: Option<DomNodeHandle>);
    fn listen(
        &self,
        node: DomNodeHandle,
        event: String,
        options: DomEventOptions,
        listener: DomEventListener,
    );
    fn remove(&self, node: DomNodeHandle);

    #[cfg(target_arch = "wasm32")]
    fn html_element(&self, node: DomNodeHandle) -> web_sys::HtmlElement;
}

/// Renderer selected for a DOM subtree.
#[derive(Clone)]
pub struct DomRendererContext {
    renderer: Rc<dyn DomRenderer>,
}

impl DomRendererContext {
    /// Creates a renderer that sends mutations to a managed native web view.
    pub fn remote(runtime: Rc<EmbeddedDomRuntime>) -> Self {
        Self { renderer: runtime }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn browser(target: web_sys::Node) -> Self {
        Self {
            renderer: crate::browser_renderer::BrowserDomRenderer::new(target),
        }
    }

    pub(crate) fn renderer(&self) -> Rc<dyn DomRenderer> {
        self.renderer.clone()
    }
}

impl DomRenderer for EmbeddedDomRuntime {
    fn root_handle(&self) -> DomNodeHandle {
        self.root_handle()
    }

    fn create_element(&self, tag: &str) -> DomNodeHandle {
        self.create_element(tag)
    }

    fn set_text(&self, node: DomNodeHandle, value: String) {
        self.set_text(node, value);
    }

    fn replace_styles(&self, node: DomNodeHandle, styles: Vec<DomStyle>) {
        self.replace_styles(node, styles);
    }

    fn set_attribute(&self, node: DomNodeHandle, name: String, value: Option<String>) {
        self.set_attribute(node, name, value);
    }

    fn set_property(&self, node: DomNodeHandle, name: String, value: DomValue) {
        self.set_property(node, name, value);
    }

    fn remove_property(&self, node: DomNodeHandle, name: String) {
        self.remove_property(node, name);
    }

    fn place(
        &self,
        node: DomNodeHandle,
        parent: DomNodeHandle,
        predecessor: Option<DomNodeHandle>,
    ) {
        self.place(node, parent, predecessor);
    }

    fn listen(
        &self,
        node: DomNodeHandle,
        event: String,
        options: DomEventOptions,
        listener: DomEventListener,
    ) {
        self.listen_with_options(node, event, options, listener);
    }

    fn remove(&self, node: DomNodeHandle) {
        self.remove(node);
    }

    #[cfg(target_arch = "wasm32")]
    fn html_element(&self, _: DomNodeHandle) -> web_sys::HtmlElement {
        panic!("remote DOM nodes do not expose browser HtmlElement handles")
    }
}

pub(crate) fn renderer(element: &Element) -> Rc<dyn DomRenderer> {
    element
        .context::<DomRendererContext>()
        .expect("DOM components must be mounted beneath a DOM renderer")
        .renderer()
}

pub(crate) fn mount_host(element: &Element, renderer: Rc<dyn DomRenderer>, node: DomNodeHandle) {
    element.provide_handle(node);
    element.on_place({
        let renderer = renderer.clone();
        move |placement| {
            let parent = placement
                .parent
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<DomNodeHandle>())
                .copied();
            let predecessor = placement
                .pred
                .as_ref()
                .and_then(|handle| handle.downcast_ref::<DomNodeHandle>())
                .copied();
            if let Some(parent) = parent {
                renderer.place(node, parent, predecessor);
            }
        }
    });
    element.on_unmount(move || renderer.remove(node));
}
