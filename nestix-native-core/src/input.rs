use nestix::{Shared, props};

#[props]
#[derive(Debug, Clone)]
pub struct InputProps {
    #[props(default)]
    pub value: String,

    #[props(default = 0.0)]
    pub x: f64,
    #[props(default = 0.0)]
    pub y: f64,

    #[props(default = 100.0)]
    pub width: f64,
    #[props(default = 24.0)]
    pub height: f64,

    pub on_text_change: Option<Shared<dyn Fn(&str)>>,
}
