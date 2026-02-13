pub mod root;
pub mod button;
pub mod input;
pub mod label;
pub mod linear_view;
pub mod dimension;
pub mod stack_view;
pub mod tab_view;
pub mod view;
pub mod window;

pub use root::*;
pub use button::*;
pub use input::*;
pub use label::*;
pub use linear_view::*;
pub use dimension::*;
pub use stack_view::*;
pub use tab_view::*;
pub use view::*;
pub use window::*;
pub use dpi;

use nestix::Element;

pub trait Backend {
    fn create_root(&self, props: RootProps) -> Option<Element>;

    fn create_button(&self, props: ButtonProps) -> Option<Element>;

    fn create_input(&self, props: InputProps) -> Option<Element>;

    fn create_label(&self, props: LabelProps) -> Option<Element>;

    fn create_linear_view(&self, props: LinearViewProps) -> Option<Element>;

    fn create_stack_view(&self, props: StackViewProps) -> Option<Element>;

    fn create_tab_view(&self, props: TabViewProps) -> Option<Element>;

    fn create_tab_view_item(&self, props: TabViewItemProps) -> Option<Element>;

    fn create_window(&self, props: WindowProps) -> Option<Element>;
}
