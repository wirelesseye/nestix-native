pub mod button;
pub mod input;
pub mod label;
pub mod linear_view;
pub mod root;
pub mod stack_view;
pub mod tab_view;
pub mod window;

pub use button::*;
pub use input::*;
pub use label::*;
pub use linear_view::*;
pub use root::*;
pub use stack_view::*;
pub use tab_view::*;
pub use window::*;

use nestix::{Shared, create_element};
use nestix_native_core::Backend;
use objc2::rc::Retained;
use objc2_foundation::NSObject;

#[derive(Clone)]
pub(crate) struct ParentContext {
    pub ns_object: Option<Retained<NSObject>>,
    pub add_child: Option<Shared<dyn Fn(&NSObject)>>,
}

pub struct AppkitBackend;

impl Backend for AppkitBackend {
    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<nestix::Element> {
        Some(create_element::<Root>(props))
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

    fn create_linear_view(
        &self,
        props: nestix_native_core::LinearViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<LinearView>(props))
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
