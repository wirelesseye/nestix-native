use nestix::{Element, props};

#[props]
#[derive(Debug, Clone)]
pub struct StackViewProps {
    pub children: Option<Vec<Element>>,
}
