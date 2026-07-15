pub mod appearance;
pub mod button;
pub mod color;
pub mod container;
pub mod contexts;
pub mod dimension;
pub mod flex_view;
pub mod font;
pub mod image_view;
pub mod input;
pub mod menu;
pub mod root;
pub mod scroll_view;
pub mod style;
pub mod tab_view;
pub mod text;
pub mod utils;
pub mod view;
pub mod window;

pub use appearance::*;
pub use button::*;
pub use color::*;
pub use container::*;
pub use contexts::*;
pub use dimension::*;
pub use flex_view::*;
pub use font::*;
pub use image_view::*;
pub use input::*;
pub use menu::*;
pub use root::*;
pub use scroll_view::*;
pub use style::*;
pub use tab_view::*;
pub use text::*;
pub use utils::*;
pub use view::*;
pub use window::*;

pub use dpi;
pub use nestix_native_macros::*;

use nestix::Element;

pub trait Backend {
    fn backend_id(&self) -> &'static str;

    fn create_root(&self, _props: RootProps) -> Option<Element> {
        None
    }

    fn create_scroll_view(&self, _props: ScrollViewProps) -> Option<Element> {
        None
    }

    fn create_button(&self, _props: ButtonProps) -> Option<Element> {
        None
    }

    fn create_flex_view(&self, _props: FlexViewProps) -> Option<Element> {
        None
    }

    fn create_input(&self, _props: InputProps) -> Option<Element> {
        None
    }

    fn create_image_view(&self, _props: ImageViewProps) -> Option<Element> {
        None
    }

    fn create_text(&self, _props: TextProps) -> Option<Element> {
        None
    }

    fn create_tab_view(&self, _props: TabViewProps) -> Option<Element> {
        None
    }

    fn create_tab_view_item(&self, _props: TabViewItemProps) -> Option<Element> {
        None
    }

    fn create_window(&self, _props: WindowProps) -> Option<Element> {
        None
    }

    fn create_menu(&self, _props: MenuProps) -> Option<Element> {
        None
    }
    fn create_menu_bar(&self, _props: MenuBarProps) -> Option<Element> {
        None
    }
    fn create_submenu(&self, _props: SubmenuProps) -> Option<Element> {
        None
    }
    fn create_menu_item(&self, _props: MenuItemProps) -> Option<Element> {
        None
    }
    fn create_check_menu_item(&self, _props: CheckMenuItemProps) -> Option<Element> {
        None
    }
    fn create_radio_menu_item(&self, _props: RadioMenuItemProps) -> Option<Element> {
        None
    }
    fn create_menu_separator(&self, _props: MenuSeparatorProps) -> Option<Element> {
        None
    }

    /// The default preserves the wrapped visual target on unsupported backends.
    fn create_context_menu(&self, props: ContextMenuProps) -> Option<Element> {
        Some(props.children.get())
    }
}
