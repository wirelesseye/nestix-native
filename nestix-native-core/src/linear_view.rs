use nestix::{Layout, props};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearViewDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearViewAlignment {
    Unset,
    Start,
    End,
    Center,
}

#[props]
#[derive(Debug, Clone)]
pub struct LinearViewProps {
    #[props(default = LinearViewDirection::Vertical)]
    pub direction: LinearViewDirection,
    #[props(default = LinearViewAlignment::Unset)]
    pub alignment: LinearViewAlignment,
    #[props(default)]
    pub children: Layout,
}
