//! Cross-platform Nestix components backed by native platform controls.

/// Defines facade components which forward their props to a backend factory.
///
/// The macro accepts one or more component mappings so modules containing a
/// family of related components can declare them together.
macro_rules! delegate {
    (
        $(
            $(#[$attribute:meta])*
            $visibility:vis $component:ident($props:path) => $factory:ident
        ),+ $(,)?
    ) => {
        $(
            $(#[$attribute])*
            #[nestix::component]
            $visibility fn $component(
                props: &$props,
                element: &nestix::Element,
            ) -> Option<nestix::Element> {
                element
                    .context::<crate::BackendContext>()
                    .expect("native components must be mounted beneath Root")
                    .backend
                    .$factory(props.clone())
            }
        )+
    };
}

pub mod backend_override;
pub mod button;
pub mod checkbox;
pub mod drag_drop;
pub mod file_picker;
pub mod flex_view;
pub mod image_view;
pub mod input;
pub mod menu;
pub mod radio_button;
pub mod root;
pub mod scroll_view;
pub mod select;
pub mod slider;
pub mod switch;
pub mod tab_view;
pub mod text;
pub mod tray_icon;
pub mod window;

pub use backend_override::*;
pub use button::*;
pub use checkbox::*;
pub use drag_drop::*;
pub use file_picker::*;
pub use flex_view::*;
pub use image_view::*;
pub use input::*;
pub use menu::*;
pub use radio_button::*;
pub use root::*;
pub use scroll_view::*;
pub use select::*;
pub use slider::*;
pub use switch::*;
pub use tab_view::*;
pub use text::*;
pub use tray_icon::*;
pub use window::*;

pub use nestix_native_core::*;

/// DOM backend APIs available to WebAssembly builds.
#[cfg(all(target_arch = "wasm32", feature = "dom"))]
pub mod dom {
    pub use nestix_native_dom::*;
}

/// Returns the backend selected for browser WebAssembly builds.
#[cfg(all(target_arch = "wasm32", feature = "dom"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_dom::DOM_BACKEND
}

/// Returns the backend selected for the current platform and feature set.
#[cfg(all(not(target_arch = "wasm32"), target_os = "macos", feature = "appkit"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_appkit::APPKIT_BACKEND
}

/// Returns the backend selected for the current platform and feature set.
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows", feature = "win32"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_win32::WIN32_BACKEND
}

/// Returns the backend selected for the current platform and feature set.
#[cfg(all(not(target_arch = "wasm32"), target_os = "linux", feature = "gtk4"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_gtk4::GTK4_BACKEND
}

/// Returns the backend selected for the current platform and feature set.
///
/// # Panics
///
/// Panics when no backend feature is enabled for the target platform.
#[cfg(not(any(
    all(target_arch = "wasm32", feature = "dom"),
    all(not(target_arch = "wasm32"), target_os = "macos", feature = "appkit"),
    all(not(target_arch = "wasm32"), target_os = "windows", feature = "win32"),
    all(not(target_arch = "wasm32"), target_os = "linux", feature = "gtk4")
)))]
pub fn default_backend() -> &'static dyn Backend {
    panic!(
        "nestix-native has no default backend for this build; enable the platform feature or provide a BackendContext"
    )
}

/// Context that selects the backend used by descendant native components.
#[derive(Clone)]
pub struct BackendContext {
    /// Backend responsible for constructing native controls.
    pub backend: &'static dyn Backend,
}
