use nestix::derive_props;

#[derive_props]
#[derive(Debug, Clone)]
pub struct LabelProps {
    pub text: String,

    #[props(default = 0.0)]
    pub x: f64,
    #[props(default = 0.0)]
    pub y: f64,

    #[props(default = 100.0)]
    pub width: f64,
    #[props(default = 24.0)]
    pub height: f64,
}
