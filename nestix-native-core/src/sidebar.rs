use nestix::{Element, Shared, props};

/// Properties for a sidebar attached to the nearest containing window.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct SidebarProps {
    /// Preferred width of the expanded sidebar in logical pixels.
    pub width: Option<f64>,

    /// Minimum width of the expanded sidebar in logical pixels.
    pub min_width: Option<f64>,

    /// Whether the user may resize the sidebar by dragging its divider.
    #[props(default = true)]
    pub resizable: bool,

    /// Whether the sidebar is open.
    ///
    /// When omitted, the native backend owns the sidebar's open state.
    pub open: Option<bool>,

    /// Called when native interaction requests a change to the sidebar's open state.
    pub on_open_change: Option<Shared<dyn Fn(bool)>>,

    /// Optional content displayed in the sidebar pane.
    pub children: Option<Element>,
}
