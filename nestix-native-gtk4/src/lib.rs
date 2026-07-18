pub mod button;
pub mod flex_view;
pub mod root;
pub mod text;
pub mod window;

mod contexts;
mod layout;

pub use button::*;
pub use flex_view::*;
pub use root::*;
pub use text::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

pub const GTK4_BACKEND: Gtk4Backend = Gtk4Backend;

pub struct Gtk4Backend;

impl Backend for Gtk4Backend {
    fn backend_id(&self) -> &'static str {
        "nestix-native-gtk4"
    }

    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<nestix::Element> {
        Some(create_element::<Root>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        Some(create_element::<Button>(props))
    }

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<Window>(props))
    }

    fn create_text(&self, props: nestix_native_core::TextProps) -> Option<nestix::Element> {
        Some(create_element::<Text>(props))
    }

    fn create_flex_view(
        &self,
        props: nestix_native_core::FlexViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<FlexView>(props))
    }
}
