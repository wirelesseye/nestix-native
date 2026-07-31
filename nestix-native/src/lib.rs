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
                crate::create_backend_element(element, stringify!($component), |backend| {
                    backend.$factory(props.clone())
                })
            }
        )+
    };
}

pub mod backend_case;
pub mod button;
pub mod checkbox;
#[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
pub mod dom_surface;
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
pub mod sidebar;
pub mod slider;
pub mod switch;
pub mod tab_view;
pub mod text;
pub mod tray_icon;
pub mod web_view;
pub mod window;

pub use backend_case::*;
pub use button::*;
pub use checkbox::*;
#[cfg(all(feature = "dom", not(target_arch = "wasm32")))]
pub use dom_surface::*;
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
pub use sidebar::*;
pub use slider::*;
pub use switch::*;
pub use tab_view::*;
pub use text::*;
pub use tray_icon::*;
pub use web_view::*;
pub use window::*;

pub use nestix_native_core::*;

/// AppKit backend APIs available to macOS builds.
#[cfg(all(target_os = "macos", feature = "appkit"))]
pub mod appkit {
    pub use nestix_native_appkit::*;
}

/// DOM backend and managed-surface APIs.
#[cfg(feature = "dom")]
pub mod dom {
    pub use nestix_native_dom::*;
}

/// GTK4 backend APIs available to Linux builds.
#[cfg(all(target_os = "linux", feature = "gtk4"))]
pub mod gtk4 {
    pub use nestix_native_gtk4::*;
}

/// Win32 backend APIs available to Windows builds.
#[cfg(all(target_os = "windows", feature = "win32"))]
pub mod win32 {
    pub use nestix_native_win32::*;
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
