pub mod app;
pub mod button;
pub mod label;
pub mod view;
pub mod window;

pub use app::*;
pub use button::*;
pub use label::*;
pub use view::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

pub struct AppkitBackend;

impl Backend for AppkitBackend {
    fn create_app(&self, props: nestix_native_core::AppProps) -> Option<nestix::Element> {
        Some(create_element::<AppkitApp>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        Some(create_element::<AppkitButton>(props))
    }

    fn create_label(&self, props: nestix_native_core::LabelProps) -> Option<nestix::Element> {
        Some(create_element::<AppkitLabel>(props))
    }

    fn create_list_view(
        &self,
        props: nestix_native_core::ListViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<AppkitListView>(props))
    }

    fn create_stack_view(
        &self,
        props: nestix_native_core::StackViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<AppkitStackView>(props))
    }

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<AppkitWindow>(props))
    }
}
