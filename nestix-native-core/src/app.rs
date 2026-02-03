use nestix::{Layout, props};

#[props]
#[derive(Debug, Clone)]
pub struct AppProps {
    #[props(default)]
    pub children: Layout,

    #[props(default = true)]
    pub quit_when_all_windows_closed: bool,
}
