use nestix::{Layout, props};

#[props]
#[derive(Debug, Clone)]
pub struct StackViewProps {
    #[props(default)]
    pub children: Layout,
}
