use nestix::{Element, Shared, props};

use crate::ClassList;

/// Controls how a window's native title bar is presented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TitleBarMode {
    /// Use the platform's standard title bar and window controls.
    #[default]
    System,

    /// Hide the title bar and its window controls.
    Hidden,

    /// Extend window content into the title-bar area while retaining the
    /// platform's window controls.
    Overlay,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct WindowProps {
    #[props(default)]
    pub class: ClassList,

    pub children: Option<Element>,

    #[props(default)]
    pub title: String,

    #[props(default)]
    pub title_bar_mode: TitleBarMode,

    #[props(default = 800.0)]
    pub width: f64,
    #[props(default = 600.0)]
    pub height: f64,

    pub on_resize: Option<Shared<dyn Fn(dpi::Size)>>,

    /// Called when the user asks to close the window. The native window stays
    /// open until this component is unmounted.
    pub on_close_requested: Option<Shared<dyn Fn()>>,
}
