pub mod button;
pub mod flex_view;
pub mod input;
pub mod root;
pub mod tab_view;
pub mod text;
pub mod window;

mod allocation_bin;
mod contexts;
mod layout;

pub use button::*;
pub use flex_view::*;
pub use input::*;
pub use root::*;
pub use tab_view::*;
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

    fn create_input(&self, props: nestix_native_core::InputProps) -> Option<nestix::Element> {
        Some(create_element::<Input>(props))
    }

    fn create_tab_view(&self, props: nestix_native_core::TabViewProps) -> Option<nestix::Element> {
        Some(create_element::<TabView>(props))
    }

    fn create_tab_view_item(
        &self,
        props: nestix_native_core::TabViewItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<TabViewItem>(props))
    }
}
