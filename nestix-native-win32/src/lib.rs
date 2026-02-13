pub mod label;
pub mod root;
pub mod stack_view;
pub mod window;

pub use label::*;
pub use root::*;
pub use stack_view::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;
use windows::Win32::Foundation::HWND;

#[derive(Clone)]
pub struct ParentContext {
    pub hwnd: Option<HWND>,
}

pub struct Win32Backend;

impl Backend for Win32Backend {
    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<nestix::Element> {
        Some(create_element::<Root>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        None
    }

    fn create_input(&self, props: nestix_native_core::InputProps) -> Option<nestix::Element> {
        None
    }

    fn create_label(&self, props: nestix_native_core::LabelProps) -> Option<nestix::Element> {
        Some(create_element::<Label>(props))
    }

    fn create_linear_view(
        &self,
        props: nestix_native_core::LinearViewProps,
    ) -> Option<nestix::Element> {
        None
    }

    fn create_stack_view(
        &self,
        props: nestix_native_core::StackViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<StackView>(props))
    }

    fn create_tab_view(&self, props: nestix_native_core::TabViewProps) -> Option<nestix::Element> {
        None
    }

    fn create_tab_view_item(
        &self,
        props: nestix_native_core::TabViewItemProps,
    ) -> Option<nestix::Element> {
        None
    }

    fn create_window(&self, props: nestix_native_core::WindowProps) -> Option<nestix::Element> {
        Some(create_element::<Window>(props))
    }
}
