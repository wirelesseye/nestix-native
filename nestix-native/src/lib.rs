pub mod button;
pub mod flex_view;
pub mod input;
pub mod root;
pub mod scroll_view;
pub mod tab_view;
pub mod text;
pub mod window;

pub use button::*;
pub use flex_view::*;
pub use input::*;
pub use root::*;
pub use scroll_view::*;
pub use tab_view::*;
pub use text::*;
pub use window::*;

pub use nestix_native_core::*;

#[cfg(all(target_os = "macos", feature = "appkit"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_appkit::APPKIT_BACKEND
}

#[cfg(all(target_os = "windows", feature = "win32"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_win32::WIN32_BACKEND
}

#[cfg(not(any(
    all(target_os = "macos", feature = "appkit"),
    all(target_os = "windows", feature = "win32")
)))]
pub fn default_backend() -> &'static dyn Backend {
    panic!(
        "nestix-native has no default backend for this build; enable the platform feature or provide a BackendContext"
    )
}

#[derive(Clone)]
pub struct BackendContext {
    pub backend: &'static dyn Backend,
}
