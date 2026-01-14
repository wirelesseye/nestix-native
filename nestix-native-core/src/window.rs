use nestix::{Element, props};

#[props]
#[derive(Debug, Clone)]
pub struct WindowProps {
    pub view: Option<Element>,

    #[props(default)]
    pub title: String,

    #[props(default = 800.0)]
    pub width: f64,
    #[props(default = 600.0)]
    pub height: f64,
}
