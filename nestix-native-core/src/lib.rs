pub mod button;
pub mod color;
pub mod dimension;
pub mod flex_view;
pub mod input;
pub mod label;
pub mod root;
pub mod tab_view;
pub mod view;
pub mod window;

pub use button::*;
pub use color::*;
pub use dimension::*;
pub use flex_view::*;
pub use input::*;
pub use label::*;
pub use root::*;
pub use tab_view::*;
pub use view::*;
pub use window::*;

pub use dpi;

use nestix::Element;

pub trait Backend {
    fn create_root(&self, props: RootProps) -> Option<Element>;

    fn create_button(&self, props: ButtonProps) -> Option<Element>;

    fn create_flex_view(&self, props: FlexViewProps) -> Option<Element>;

    fn create_input(&self, props: InputProps) -> Option<Element>;

    fn create_label(&self, props: LabelProps) -> Option<Element>;

    fn create_tab_view(&self, props: TabViewProps) -> Option<Element>;

    fn create_tab_view_item(&self, props: TabViewItemProps) -> Option<Element>;

    fn create_window(&self, props: WindowProps) -> Option<Element>;
}
