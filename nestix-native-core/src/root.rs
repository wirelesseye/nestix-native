use nestix::{Layout, props};

#[props(debug)]
#[derive(Debug, Clone)]
pub struct RootProps {
    #[props(default)]
    pub children: Layout,

    #[props(default = true)]
    pub quit_when_all_windows_closed: bool,
}
