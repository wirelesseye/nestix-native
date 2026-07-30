use std::{
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use nestix::{
    ComponentOutput, DetachedTree, Element, component, components::ContextProvider, layout,
};
use nestix_native_core::WebViewBridge;
use nestix_native_dom::{
    DOM_BACKEND_ID, DOM_SURFACE_BACKEND, DOM_SURFACE_BACKEND_ID, DomDocumentRoot,
    DomRendererContext, DomSurfaceId, EmbeddedDomRuntime, ManagedDomBridge, dom_template_source,
};

pub use nestix_native_core::DomSurfaceProps;

use crate::{BackendContext, WebView};

static NEXT_SURFACE_ID: AtomicU64 = AtomicU64::new(1);

/// Embeds a managed DOM document and renders descendants with the surface DOM backend.
#[component]
pub fn DomSurface(props: &DomSurfaceProps, element: &Element) -> Element {
    let host_backend = element
        .context::<BackendContext>()
        .expect("DomSurface must be mounted beneath Root")
        .backend;

    let host_backend_id = host_backend.backend_id();
    assert_ne!(
        host_backend_id, DOM_BACKEND_ID,
        "DomSurface cannot be nested inside the browser DOM backend"
    );
    assert_ne!(
        host_backend_id, DOM_SURFACE_BACKEND_ID,
        "DomSurface cannot be nested inside another DomSurface"
    );

    let runtime = EmbeddedDomRuntime::new(DomSurfaceId(
        NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed),
    ));
    let source = dom_template_source(props.template.get());
    let bridge: Rc<dyn WebViewBridge> = ManagedDomBridge::new(runtime.clone());

    // Keep the managed document lifecycle-owned by DomSurface without placing
    // the WebView in a private list that would hide DomSurface's predecessor.
    let managed_tree = layout! {
        DetachedTree {
            ContextProvider<nestix_native_core::NativeVisualMount>(
                nestix_native_core::NativeVisualMount::blocked("DomSurface"),
            ) {
                ContextProvider<DomRendererContext>(DomRendererContext::remote(runtime)) {
                    ContextProvider<BackendContext>(
                        BackendContext { backend: &DOM_SURFACE_BACKEND,  },
                    ) {
                        DomDocumentRoot(.children = props.children.clone())
                    }
                }
            }
        }
    };
    managed_tree.mount(Some(element));

    layout! {
        WebView(
            source,
            .class = props.class.clone(),
            .view = props.view.clone(),
            .transparent = props.transparent.clone(),
            .inspectable = props.inspectable.clone(),
            .controller = props.controller.clone(),
            .bridge = Some(bridge),
        )
    }
}
