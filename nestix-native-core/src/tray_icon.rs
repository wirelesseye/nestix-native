use std::fmt;

use nestix::{Element, Shared, props};

use crate::ImageSource;

/// Error returned when a tray activation event cannot present its menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconError {
    /// The tray icon which produced the event is no longer mounted or visible.
    NotMounted,
    /// The tray icon does not currently have a mounted menu.
    MenuUnavailable,
    /// The native backend could not begin menu presentation.
    PresentationFailed,
}

impl fmt::Display for TrayIconError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted => formatter.write_str("tray icon is not mounted"),
            Self::MenuUnavailable => formatter.write_str("tray icon has no menu"),
            Self::PresentationFailed => formatter.write_str("tray icon menu could not be shown"),
        }
    }
}

impl std::error::Error for TrayIconError {}

/// An activation of a mounted tray icon.
///
/// Call [`show_menu`](Self::show_menu) from either activation callback to
/// choose which native interaction presents the tray icon's menu.
#[derive(Clone)]
pub struct TrayIconEvent {
    show_menu: Shared<dyn Fn() -> Result<(), TrayIconError>>,
}

impl fmt::Debug for TrayIconEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrayIconEvent")
            .finish_non_exhaustive()
    }
}

impl TrayIconEvent {
    pub fn show_menu(&self) -> Result<(), TrayIconError> {
        (self.show_menu)()
    }

    #[doc(hidden)]
    pub fn new(show_menu: Shared<dyn Fn() -> Result<(), TrayIconError>>) -> Self {
        Self { show_menu }
    }
}

/// Props for an application-level notification-area or status-bar icon.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TrayIconProps {
    pub icon: ImageSource,
    pub tooltip: Option<String>,
    pub menu: Option<Element>,
    #[props(default = true)]
    pub visible: bool,
    #[props(default = true)]
    pub template: bool,
    pub on_activate: Option<Shared<dyn Fn(TrayIconEvent)>>,
    pub on_secondary: Option<Shared<dyn Fn(TrayIconEvent)>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn event_forwards_menu_result() {
        let event = TrayIconEvent::new(Shared::from(
            Rc::new(|| Err(TrayIconError::MenuUnavailable))
                as Rc<dyn Fn() -> Result<(), TrayIconError>>,
        ));

        assert_eq!(event.show_menu(), Err(TrayIconError::MenuUnavailable));
    }
}
