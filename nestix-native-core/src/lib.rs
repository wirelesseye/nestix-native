//! Platform-independent native component props, styles, and backend contracts.

pub mod animation;
pub mod appearance;
pub mod backend_provider;
pub mod button;
pub mod checkbox;
pub mod color;
pub mod container;
pub mod contexts;
pub mod dimension;
pub mod dom_surface;
pub mod drag_drop;
pub mod file_picker;
pub mod flex_view;
pub mod font;
pub mod image_view;
pub mod input;
pub mod menu;
pub mod navigation;
pub mod radio_button;
pub mod root;
pub mod scroll_view;
pub mod select;
pub mod sidebar;
pub mod slider;
pub mod style;
pub mod switch;
pub mod tab_view;
pub mod text;
pub mod tray_icon;
pub mod utils;
pub mod view;
pub mod web_view;
pub mod window;

pub use animation::*;
pub use appearance::*;
pub use backend_provider::*;
pub use button::*;
pub use checkbox::*;
pub use color::*;
pub use container::*;
pub use contexts::*;
pub use dimension::*;
pub use dom_surface::*;
pub use drag_drop::*;
pub use file_picker::*;
pub use flex_view::*;
pub use font::*;
pub use image_view::*;
pub use input::*;
pub use menu::*;
pub use navigation::*;
pub use radio_button::*;
pub use root::*;
pub use scroll_view::*;
pub use select::*;
pub use sidebar::*;
pub use slider::*;
pub use style::*;
pub use switch::*;
pub use tab_view::*;
pub use text::*;
pub use tray_icon::*;
pub use utils::*;
pub use view::*;
pub use web_view::*;
pub use window::*;

pub use dpi;
pub use nestix_native_macros::*;

use nestix::{Component, Element, create_element};

/// Controls whether native visual components may mount in a logical subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVisualMount {
    /// Native visuals from this backend may mount in the subtree.
    Allowed { backend_id: &'static str },
    /// No native visual may cross this renderer boundary.
    Blocked { boundary: &'static str },
}

impl NativeVisualMount {
    pub const fn allowed(backend_id: &'static str) -> Self {
        Self::Allowed { backend_id }
    }

    pub const fn blocked(boundary: &'static str) -> Self {
        Self::Blocked { boundary }
    }
}

/// Checks the visual-mount policy inherited by a concrete backend component.
///
/// A missing policy permits mounting so concrete backend crates remain usable
/// without the `nestix-native` facade. Platform roots should provide an
/// [`NativeVisualMount::Allowed`] policy to prevent visuals from another
/// backend from crossing into their native tree.
pub fn native_visual_mount_allowed(
    element: &Element,
    backend_id: &'static str,
    component: &'static str,
) -> bool {
    match element.context::<NativeVisualMount>().as_deref() {
        None => true,
        Some(NativeVisualMount::Allowed {
            backend_id: allowed,
        }) if *allowed == backend_id => true,
        Some(NativeVisualMount::Allowed {
            backend_id: allowed,
        }) => {
            log::warn!(
                "{component} from backend {backend_id} was not mounted in visual tree owned by {allowed}"
            );
            false
        }
        Some(NativeVisualMount::Blocked { boundary }) => {
            log::warn!(
                "{component} from backend {backend_id} was not mounted across {boundary} boundary"
            );
            false
        }
    }
}

struct EmptyVisualOutput;

impl Component for EmptyVisualOutput {
    type Props = ();

    fn on_mount(_: &Element) {}
}

#[doc(hidden)]
pub fn empty_visual_output() -> Element {
    create_element::<EmptyVisualOutput>(())
}

/// Context that selects the backend used by descendant facade components.
#[derive(Clone)]
pub struct BackendContext {
    /// Backend responsible for constructing controls in this subtree.
    pub backend: &'static dyn Backend,
}

/// Factory interface implemented by native platform backends.
pub trait Backend {
    /// Returns a stable identifier for this backend.
    fn backend_id(&self) -> &'static str;

    /// Creates the root of a native component tree.
    fn create_root(&self, _props: RootProps) -> Option<Element> {
        None
    }

    /// Creates a scrollable container.
    fn create_scroll_view(&self, props: ScrollViewProps) -> Option<Element> {
        props.children.get()
    }

    /// Creates a push button.
    fn create_button(&self, _props: ButtonProps) -> Option<Element> {
        None
    }

    /// Creates a checkbox.
    fn create_checkbox(&self, _props: CheckboxProps) -> Option<Element> {
        None
    }

    /// Creates a radio button.
    fn create_radio_button(&self, _props: RadioButtonProps) -> Option<Element> {
        None
    }

    /// Creates an on/off switch.
    fn create_switch(&self, _props: SwitchProps) -> Option<Element> {
        None
    }

    /// Creates a selection control.
    fn create_select(&self, _props: SelectProps) -> Option<Element> {
        None
    }

    /// Creates an option belonging to a selection control.
    fn create_select_option(&self, _props: SelectOptionProps) -> Option<Element> {
        None
    }

    /// Attaches a sidebar to the nearest containing window.
    fn create_sidebar(&self, _props: SidebarProps) -> Option<Element> {
        None
    }

    /// Creates a navigation list for use in a sidebar.
    fn create_sidebar_navigation(&self, _props: SidebarNavigationProps) -> Option<Element> {
        None
    }

    /// Creates an item belonging to a sidebar navigation list.
    fn create_navigation_item(&self, _props: NavigationItemProps) -> Option<Element> {
        None
    }

    /// Creates a numeric slider.
    fn create_slider(&self, _props: SliderProps) -> Option<Element> {
        None
    }

    /// Creates a flex-layout container.
    fn create_flex_view(&self, _props: FlexViewProps) -> Option<Element> {
        None
    }

    /// The default preserves the wrapped visual target on unsupported backends.
    fn create_drag_source(&self, props: DragSourceProps) -> Option<Element> {
        Some(props.children.get())
    }

    /// The default preserves the wrapped visual target on unsupported backends.
    fn create_drop_target(&self, props: DropTargetProps) -> Option<Element> {
        Some(props.children.get())
    }

    /// Creates a file-picker service component.
    fn create_file_picker(&self, _props: FilePickerProps) -> Option<Element> {
        None
    }

    /// Creates a text input.
    fn create_input(&self, _props: InputProps) -> Option<Element> {
        None
    }

    /// Creates an image view.
    fn create_image_view(&self, _props: ImageViewProps) -> Option<Element> {
        None
    }

    /// Creates a view that displays web content.
    fn create_web_view(&self, _props: WebViewProps) -> Option<Element> {
        None
    }

    /// Creates a text label.
    fn create_text(&self, _props: TextProps) -> Option<Element> {
        None
    }

    /// Creates a tabbed container.
    fn create_tab_view(&self, _props: TabViewProps) -> Option<Element> {
        None
    }

    /// Creates a page belonging to a tabbed container.
    fn create_tab_view_item(&self, _props: TabViewItemProps) -> Option<Element> {
        None
    }

    /// Creates a top-level window.
    fn create_window(&self, _props: WindowProps) -> Option<Element> {
        None
    }

    /// Installs a menu as a window menu bar.
    fn create_menu_bar(&self, _props: MenuBarProps) -> Option<Element> {
        None
    }
    /// The default preserves the wrapped visual target on unsupported backends.
    fn create_context_menu(&self, props: ContextMenuProps) -> Option<Element> {
        Some(props.children.get())
    }

    /// Creates a notification-area or status-bar icon.
    fn create_tray_icon(&self, _props: TrayIconProps) -> Option<Element> {
        None
    }
}
