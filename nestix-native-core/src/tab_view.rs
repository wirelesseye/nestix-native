use nestix::{Element, derive_props};

#[derive_props]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    pub children: Option<Vec<Element>>,
}

#[derive_props]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    pub id: String,
    pub view: Option<Element>,
    #[props(default)]
    pub title: String,
}
