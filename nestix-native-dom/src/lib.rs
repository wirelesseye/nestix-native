//! Browser DOM backend for Nestix Native.

#[cfg(target_arch = "wasm32")]
mod browser_renderer;
mod button;
#[cfg(target_arch = "wasm32")]
mod dom;
#[cfg(target_arch = "wasm32")]
mod dom_element;
mod flex_view;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod remote;
mod renderer;
#[cfg(target_arch = "wasm32")]
mod root;
mod runtime;
#[cfg(target_arch = "wasm32")]
mod style;
mod style_declarations;
mod text;
#[cfg(target_arch = "wasm32")]
mod web_view;
#[cfg(target_arch = "wasm32")]
mod window;

#[cfg(all(test, target_arch = "wasm32"))]
mod tests;

pub use button::*;
#[cfg(target_arch = "wasm32")]
pub use dom_element::*;
pub use flex_view::*;
pub use input::*;
#[cfg(not(target_arch = "wasm32"))]
pub use remote::*;
pub use renderer::DomRendererContext;
#[cfg(target_arch = "wasm32")]
pub use root::*;
pub use runtime::*;
pub use text::*;
#[cfg(target_arch = "wasm32")]
pub use web_view::*;
#[cfg(target_arch = "wasm32")]
pub use window::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

use nestix::{Element, create_element};
use nestix_native_core::Backend;

/// Shared DOM backend instance.
pub const DOM_BACKEND: DomBackend = DomBackend;

/// Backend that renders Nestix Native components into browser DOM nodes.
pub struct DomBackend;

impl Backend for DomBackend {
    fn backend_id(&self) -> &'static str {
        "nestix-native-dom"
    }

    #[cfg(target_arch = "wasm32")]
    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<Element> {
        Some(create_element::<Root>(props))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<Element> {
        Some(create_element::<Window>(props))
    }

    fn create_flex_view(&self, props: nestix_native_core::FlexViewProps) -> Option<Element> {
        Some(create_element::<FlexView>(props))
    }

    fn create_text(&self, props: nestix_native_core::TextProps) -> Option<Element> {
        Some(create_element::<Text>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<Element> {
        Some(create_element::<Button>(props))
    }

    fn create_input(&self, props: nestix_native_core::InputProps) -> Option<Element> {
        Some(create_element::<Input>(props))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_web_view(&self, props: nestix_native_core::WebViewProps) -> Option<Element> {
        Some(create_element::<WebView>(props))
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_MOUNT_TARGET: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
}

/// Mounts a Nestix tree into the element matched by `selector`.
///
/// # Panics
///
/// Panics when no browser document exists, `selector` is invalid, no element
/// matches it, or the tree does not contain a Nestix Native [`Root`].
#[cfg(target_arch = "wasm32")]
pub fn mount_root(selector: &str, app: &Element) {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .expect("nestix-native-dom requires a browser document");
    let target = document
        .query_selector(selector)
        .unwrap_or_else(|_| panic!("invalid DOM mount selector `{selector}`"))
        .unwrap_or_else(|| panic!("DOM mount selector `{selector}` did not match an element"));

    PENDING_MOUNT_TARGET.with_borrow_mut(|pending| {
        assert!(pending.is_none(), "a DOM root mount is already pending");
        *pending = Some(target);
    });
    nestix::mount_root(app);
    assert!(
        PENDING_MOUNT_TARGET.with_borrow(|pending| pending.is_none()),
        "the mounted tree must contain a nestix_native::Root"
    );
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn take_mount_target() -> web_sys::Element {
    PENDING_MOUNT_TARGET
        .with_borrow_mut(Option::take)
        .expect("DOM Root must be mounted with nestix_native_dom::mount_root")
}
