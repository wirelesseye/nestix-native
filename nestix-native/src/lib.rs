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

#[cfg(all(target_os = "macos", feature = "appkit"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_appkit::APPKIT_BACKEND
}

#[cfg(all(target_os = "windows", feature = "win32"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_win32::WIN32_BACKEND
}

#[cfg(all(target_os = "linux", feature = "gtk4"))]
pub fn default_backend() -> &'static dyn Backend {
    &nestix_native_gtk4::GTK4_BACKEND
}

#[cfg(not(any(
    all(target_os = "macos", feature = "appkit"),
    all(target_os = "windows", feature = "win32"),
    all(target_os = "linux", feature = "gtk4")
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
