use nestix::{Element, derive_props};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListViewDirection {
    Horizontal,
    Vertical,
}


#[derive_props]
#[derive(Debug, Clone)]
pub struct ListViewProps {
    #[props(default = ListViewDirection::Vertical)]
    pub direction: ListViewDirection,
    pub children: Option<Vec<Element>>,
}
