//! Win32 backend components for Nestix Native.

/// Win32 push-button component.
pub mod button;
/// Win32 checkbox component.
pub mod checkbox;
/// Win32 drag-and-drop components.
pub mod drag_drop;
/// Win32 file-picker service component.
pub mod file_picker;
/// Win32 flex-layout container component.
pub mod flex_view;
/// Win32 image-view component.
pub mod image_view;
/// Win32 single-line text-input component.
pub mod input;
/// Win32 menu components.
pub mod menu;
/// Win32 radio-button component.
pub mod radio_button;
/// Win32 application-root component.
pub mod root;
/// Win32 scroll-view component.
pub mod scroll_view;
/// Win32 selection components.
pub mod select;
/// Win32 slider component.
pub mod slider;
/// Win32 tab-view components.
pub mod tab_view;
/// Win32 text component.
pub mod text;
/// Win32 notification-area icon component.
pub mod tray_icon;
/// Win32 top-level window component.
pub mod window;

mod contexts;
mod font;
mod native_control;
mod utils;

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
pub use tab_view::*;
pub use text::*;
pub use tray_icon::*;
pub use window::*;

use nestix::create_element;
use nestix_native_core::Backend;

/// The shared Win32 backend value used to register this backend with Nestix Native.
pub const WIN32_BACKEND: Win32Backend = Win32Backend;

/// A Nestix Native backend that creates components implemented with the Win32 API.
pub struct Win32Backend;

impl Backend for Win32Backend {
    fn backend_id(&self) -> &'static str {
        "nestix-native-win32"
    }

    fn create_root(&self, props: nestix_native_core::RootProps) -> Option<nestix::Element> {
        Some(create_element::<Root>(props))
    }

    fn create_button(&self, props: nestix_native_core::ButtonProps) -> Option<nestix::Element> {
        Some(create_element::<Button>(props))
    }

    fn create_checkbox(&self, props: nestix_native_core::CheckboxProps) -> Option<nestix::Element> {
        Some(create_element::<Checkbox>(props))
    }

    fn create_radio_button(
        &self,
        props: nestix_native_core::RadioButtonProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<RadioButton>(props))
    }

    fn create_select(&self, props: nestix_native_core::SelectProps) -> Option<nestix::Element> {
        Some(create_element::<Select>(props))
    }

    fn create_select_option(
        &self,
        props: nestix_native_core::SelectOptionProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<SelectOption>(props))
    }

    fn create_slider(&self, props: nestix_native_core::SliderProps) -> Option<nestix::Element> {
        Some(create_element::<Slider>(props))
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

    fn create_image_view(
        &self,
        props: nestix_native_core::ImageViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ImageView>(props))
    }

    fn create_scroll_view(
        &self,
        props: nestix_native_core::ScrollViewProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ScrollView>(props))
    }

    fn create_text(&self, props: nestix_native_core::TextProps) -> Option<nestix::Element> {
        Some(create_element::<Text>(props))
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

    fn create_file_picker(
        &self,
        props: nestix_native_core::FilePickerProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<FilePicker>(props))
    }

    fn create_menu(&self, props: nestix_native_core::MenuProps) -> Option<nestix::Element> {
        Some(create_element::<Menu>(props))
    }

    fn create_menu_bar(&self, props: nestix_native_core::MenuBarProps) -> Option<nestix::Element> {
        Some(create_element::<MenuBar>(props))
    }

    fn create_submenu(&self, props: nestix_native_core::SubmenuProps) -> Option<nestix::Element> {
        Some(create_element::<Submenu>(props))
    }

    fn create_menu_item(
        &self,
        props: nestix_native_core::MenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<MenuItem>(props))
    }

    fn create_check_menu_item(
        &self,
        props: nestix_native_core::CheckMenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<CheckMenuItem>(props))
    }

    fn create_radio_menu_item(
        &self,
        props: nestix_native_core::RadioMenuItemProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<RadioMenuItem>(props))
    }

    fn create_menu_separator(
        &self,
        props: nestix_native_core::MenuSeparatorProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<MenuSeparator>(props))
    }

    fn create_context_menu(
        &self,
        props: nestix_native_core::ContextMenuProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<ContextMenu>(props))
    }

    fn create_drag_source(
        &self,
        props: nestix_native_core::DragSourceProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<DragSource>(props))
    }

    fn create_drop_target(
        &self,
        props: nestix_native_core::DropTargetProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<DropTarget>(props))
    }

    fn create_tray_icon(
        &self,
        props: nestix_native_core::TrayIconProps,
    ) -> Option<nestix::Element> {
        Some(create_element::<TrayIcon>(props))
    }
}
