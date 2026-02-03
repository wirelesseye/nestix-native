use nestix::{Layout, props};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListViewDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListViewAlignment {
    Unset,
    Start,
    End,
    Center,
}

#[props]
#[derive(Debug, Clone)]
pub struct ListViewProps {
    #[props(default = ListViewDirection::Vertical)]
    pub direction: ListViewDirection,
    #[props(default = ListViewAlignment::Unset)]
    pub alignment: ListViewAlignment,
    #[props(default)]
    pub children: Layout,
}
