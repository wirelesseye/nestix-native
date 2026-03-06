use nestix::{Element, Shared, props};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct WindowProps {
    pub children: Option<Element>,

    #[props(default)]
    pub title: String,

    #[props(default = 800.0)]
    pub width: f64,
    #[props(default = 600.0)]
    pub height: f64,

    pub on_resize: Option<Shared<dyn Fn(dpi::Size)>>,
}
