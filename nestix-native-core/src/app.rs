use nestix::{Element, derive_props};

#[derive_props]
#[derive(Debug, Clone)]
pub struct AppProps {
    pub children: Option<Vec<Element>>,
    
    #[props(default = true)]
    pub quit_when_no_windows: bool,
}
