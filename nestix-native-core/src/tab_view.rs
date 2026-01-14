use nestix::{Element, props};

#[props]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    pub children: Option<Vec<Element>>,
}

#[props]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    pub id: String,
    pub view: Option<Element>,
    #[props(default)]
    pub title: String,
}
