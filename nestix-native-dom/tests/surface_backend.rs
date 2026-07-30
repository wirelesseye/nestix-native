#![cfg(not(target_arch = "wasm32"))]

use nestix_native_core::Backend;
use nestix_native_dom::{DOM_BACKEND, DOM_BACKEND_ID, DOM_SURFACE_BACKEND, DOM_SURFACE_BACKEND_ID};

#[test]
fn surface_backend_has_an_independent_identity() {
    assert_eq!(DOM_BACKEND.backend_id(), DOM_BACKEND_ID);
    assert_eq!(DOM_SURFACE_BACKEND.backend_id(), DOM_SURFACE_BACKEND_ID);
    assert_ne!(DOM_SURFACE_BACKEND.backend_id(), DOM_BACKEND.backend_id());
}
