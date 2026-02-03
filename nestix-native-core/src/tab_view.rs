use nestix::{Element, Layout, props};

#[props]
#[derive(Debug, Clone)]
pub struct TabViewProps {
    #[props(default)]
    pub children: Layout,
}

#[props]
#[derive(Debug, Clone)]
pub struct TabViewItemProps {
    pub id: String,
    pub view: Option<Element>,
    #[props(default)]
    pub title: String,
}
