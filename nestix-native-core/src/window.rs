use nestix::{Element, Shared, props};

use crate::{ClassList, Material, MaterialSource};

/// Controls how a window's native title bar is presented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TitlebarMode {
    /// Use the platform's standard title bar and window controls.
    #[default]
    System,

    /// Hide the title bar and its window controls.
    Hidden,

    /// Extend window content into the title-bar area while retaining the
    /// platform's window controls.
    Overlay,
}

/// Desktop-only properties for a top-level window.
#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct DesktopWindowProps {
    /// Initial content width in logical pixels.
    #[props(default = 800.0)]
    pub width: f64,

    /// Initial content height in logical pixels.
    #[props(default = 600.0)]
    pub height: f64,

    /// Whether the user can resize the window.
    #[props(default = true)]
    pub resizable: bool,

    /// Native title-bar presentation mode.
    #[props(default)]
    pub titlebar_mode: TitlebarMode,

    /// Composited material displayed behind the window content.
    pub material: Option<Material>,

    /// Content sampled by the composited material.
    #[props(default)]
    pub material_source: MaterialSource,

    /// Called when the user asks to close the window. The window stays open
    /// until this component is unmounted.
    pub on_close_requested: Option<Shared<dyn Fn()>>,
}

/// Properties for a top-level native window.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct WindowProps {
    /// Style classes applied to the window.
    #[props(default)]
    pub class: ClassList,

    /// Optional component displayed in the window.
    pub children: Option<Element>,

    /// Text displayed in the title bar.
    #[props(default)]
    pub title: String,

    /// Whether the window is visible.
    #[props(default = true)]
    pub visible: bool,

    /// Properties used only by desktop window backends.
    #[props(nested, default)]
    pub desktop: DesktopWindowProps,

    /// Called after the native window's content size changes.
    pub on_resize: Option<Shared<dyn Fn(dpi::Size)>>,
}
