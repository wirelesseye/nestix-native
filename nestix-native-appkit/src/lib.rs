pub mod app;
pub mod button;
pub mod input;
pub mod label;
pub mod view;
pub mod window;

pub use app::*;
pub use button::*;
pub use input::*;
pub use label::*;
pub use view::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

pub struct AppkitBackend;

impl Backend for AppkitBackend {
    fn create_app(&self, props: nestix_native_core::AppProps) -> Option<nestix::Element> {
        Some(create_element::<App>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        Some(create_element::<Button>(props))
    }

    fn create_input(&self, props: nestix_native_core::InputProps) -> Option<nestix::Element> {
        Some(create_element::<Input>(props))
    }

    fn create_label(&self, props: nestix_native_core::LabelProps) -> Option<nestix::Element> {
        Some(create_element::<Label>(props))
    }

    fn create_list_view(
        &self,
        props: nestix_native_core::ListViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ListView>(props))
    }

    fn create_stack_view(
        &self,
        props: nestix_native_core::StackViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<StackView>(props))
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

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<Window>(props))
    }
}
