pub mod app;
pub mod button;
pub mod input;
pub mod label;
pub mod list_view;
pub mod stack_view;
pub mod tab_view;
pub mod window;

pub use app::*;
pub use button::*;
pub use input::*;
pub use label::*;
pub use list_view::*;
pub use stack_view::*;
pub use tab_view::*;
pub use window::*;

use nestix::Element;

pub trait Backend {
    fn create_app(&self, props: AppProps) -> Option<Element>;

    fn create_button(&self, props: ButtonProps) -> Option<Element>;

    fn create_input(&self, props: InputProps) -> Option<Element>;

    fn create_label(&self, props: LabelProps) -> Option<Element>;

    fn create_list_view(&self, props: ListViewProps) -> Option<Element>;

    fn create_stack_view(&self, props: StackViewProps) -> Option<Element>;

    fn create_tab_view(&self, props: TabViewProps) -> Option<Element>;

    fn create_tab_view_item(&self, props: TabViewItemProps) -> Option<Element>;

    fn create_window(&self, props: WindowProps) -> Option<Element>;
}
