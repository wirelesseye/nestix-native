use std::{
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use nestix::{Element, component, components::ContextProvider, layout};
use nestix_native_core::{Backend, WebViewBridge};
use nestix_native_dom::{
    DomDocumentRoot, DomRuntimeContext, DomSurfaceId, EmbeddedDomRuntime, ManagedDomBridge,
    dom_template_source,
};

pub use nestix_native_core::DomSurfaceProps;

use crate::{BackendContext, WebView, dom::DOM_BACKEND};

static NEXT_SURFACE_ID: AtomicU64 = AtomicU64::new(1);

/// Embeds a managed DOM document and renders descendants with the DOM backend.
#[component]
pub fn DomSurface(props: &DomSurfaceProps, element: &Element) -> Element {
    let host_backend = element
        .context::<BackendContext>()
        .expect("DomSurface must be mounted beneath Root")
        .backend;

    assert_ne!(
        host_backend.backend_id(),
        DOM_BACKEND.backend_id(),
        "DomSurface cannot be nested inside a DOM backend"
    );

    let runtime = EmbeddedDomRuntime::new(DomSurfaceId(
        NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed),
    ));
    let source = dom_template_source(props.template.get());
    let bridge: Rc<dyn WebViewBridge> = ManagedDomBridge::new(runtime.clone());

    layout! {
        WebView(
            source,
            .class = props.class.clone(),
            .view = props.view.clone(),
            .transparent = props.transparent.clone(),
            .bridge = Some(bridge),
        ) {
            ContextProvider<DomRuntimeContext>(DomRuntimeContext { runtime }) {
                ContextProvider<BackendContext>(BackendContext { backend: &DOM_BACKEND }) {
                    DomDocumentRoot(.children = props.children.clone())
                }
            }
        }
    }
}
