use nestix::{Element, derive_props};

#[derive_props]
#[derive(Debug, Clone)]
pub struct StackViewProps {
    pub children: Option<Vec<Element>>,
}
