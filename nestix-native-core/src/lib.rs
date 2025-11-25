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

use nestix::Element;

pub trait Backend {
    fn create_app(&self, props: AppProps) -> Option<Element>;

    fn create_button(&self, props: ButtonProps) -> Option<Element>;

    fn create_label(&self, props: LabelProps) -> Option<Element>;

    fn create_list_view(&self, props: ListViewProps) -> Option<Element>;

    fn create_stack_view(&self, props: StackViewProps) -> Option<Element>;

    fn create_tab_view(&self, props: TabViewProps) -> Option<Element>;

    fn create_tab_view_item(&self, props: TabViewItemProps) -> Option<Element>;

    fn create_window(&self, props: WindowProps) -> Option<Element>;
}
