use std::{
    cell::RefCell,
    fmt::{self, Debug},
    path::PathBuf,
    rc::Rc,
};

use nestix::{Shared, props};

use crate::{ClassList, ViewProps};

/// Function supplied by a native web view for evaluating JavaScript.
pub type JavaScriptEvaluator = Rc<dyn Fn(&str)>;

/// Platform capabilities supplied while constructing a bridge's document-start script.
///
/// The expression must evaluate to a JavaScript function accepting one string message.
/// This keeps bridge implementations independent of platform APIs such as WebKit script
/// handlers or WebView2's `chrome.webview` object.
#[derive(Debug, Clone, Copy)]
pub struct WebViewBridgeScriptContext<'a> {
    pub post_message_expression: &'a str,
}

/// Content loaded by a web view.
#[derive(Debug, Clone, PartialEq, Eq, nestix::InspectableValue)]
pub enum WebViewSource {
    Url(String),
    Html {
        html: String,
        base_url: Option<String>,
    },
    Resource {
        path: PathBuf,
        development_path: Option<PathBuf>,
    },
}

impl WebViewSource {
    /// Creates a source that navigates to a URL.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }

    /// Creates an in-memory HTML source without a base URL.
    pub fn html(html: impl Into<String>) -> Self {
        Self::Html {
            html: html.into(),
            base_url: None,
        }
    }

    /// Creates an in-memory HTML source whose relative URLs use `base_url`.
    pub fn html_with_base_url(html: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::Html {
            html: html.into(),
            base_url: Some(base_url.into()),
        }
    }

    /// Creates a source from a logical packaged-resource path.
    pub fn resource(path: impl Into<PathBuf>) -> Self {
        Self::Resource {
            path: path.into(),
            development_path: None,
        }
    }

    /// Adds a filesystem fallback used when running without an application bundle.
    pub fn with_development_path(self, path: impl Into<PathBuf>) -> Self {
        match self {
            Self::Resource {
                path: resource_path,
                ..
            } => Self::Resource {
                path: resource_path,
                development_path: Some(path.into()),
            },
            source => source,
        }
    }
}

/// Optional native bridge installed into a web view before its document loads.
pub trait WebViewBridge: Debug {
    /// Builds a script installed before the document loads.
    fn initialization_script(&self, _context: WebViewBridgeScriptContext<'_>) -> Option<String> {
        None
    }

    /// Stable name used when a backend supports named message channels.
    fn message_channel_name(&self) -> &str;

    /// Connects the bridge to the web view's JavaScript evaluator.
    fn attach(&self, evaluate_javascript: JavaScriptEvaluator);

    /// Delivers a script message emitted by the loaded content.
    fn receive_message(&self, message: &str);

    /// Disconnects the bridge from its web view.
    fn detach(&self);
}

/// Error returned when a web view's developer tools cannot be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebViewDevToolsError {
    /// The controller is not currently connected to a mounted web view.
    NotMounted,
    /// The mounted web view has not opted into inspection.
    NotInspectable,
    /// The current backend cannot open developer tools programmatically.
    Unsupported(String),
    /// The native backend failed to open developer tools.
    Backend(String),
}

impl fmt::Display for WebViewDevToolsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted => formatter.write_str("web view is not mounted"),
            Self::NotInspectable => formatter.write_str("web view is not inspectable"),
            Self::Unsupported(message) => {
                write!(formatter, "developer tools are unsupported: {message}")
            }
            Self::Backend(message) => {
                write!(formatter, "failed to open developer tools: {message}")
            }
        }
    }
}

impl std::error::Error for WebViewDevToolsError {}

#[doc(hidden)]
#[derive(Clone)]
pub struct WebViewPresenter {
    pub open_dev_tools: Shared<dyn Fn() -> Result<(), WebViewDevToolsError>>,
}

#[derive(Default)]
struct WebViewControllerState {
    next_binding_id: u64,
    presenter: Option<(u64, WebViewPresenter)>,
}

/// Cloneable controller for imperative operations on a mounted [`WebViewProps`].
#[derive(Clone, Default)]
pub struct WebViewController {
    state: Rc<RefCell<WebViewControllerState>>,
}

impl Debug for WebViewController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebViewController")
            .field("mounted", &self.state.borrow().presenter.is_some())
            .finish()
    }
}

impl WebViewController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the platform developer tools for the mounted web view.
    pub fn open_dev_tools(&self) -> Result<(), WebViewDevToolsError> {
        let presenter = self
            .state
            .borrow()
            .presenter
            .as_ref()
            .map(|(_, presenter)| presenter.clone())
            .ok_or(WebViewDevToolsError::NotMounted)?;
        (presenter.open_dev_tools)()
    }

    #[doc(hidden)]
    pub fn bind(&self, presenter: WebViewPresenter) -> WebViewRegistration {
        let mut state = self.state.borrow_mut();
        let binding_id = state.next_binding_id;
        state.next_binding_id = state.next_binding_id.wrapping_add(1);
        state.presenter = Some((binding_id, presenter));
        WebViewRegistration {
            controller: self.clone(),
            binding_id,
        }
    }
}

#[doc(hidden)]
pub struct WebViewRegistration {
    controller: WebViewController,
    binding_id: u64,
}

impl Drop for WebViewRegistration {
    fn drop(&mut self) {
        let mut state = self.controller.state.borrow_mut();
        if state
            .presenter
            .as_ref()
            .is_some_and(|(binding_id, _)| *binding_id == self.binding_id)
        {
            state.presenter = None;
        }
    }
}

/// Properties for a native view that displays web content.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct WebViewProps {
    /// Style classes applied to the web view.
    #[props(default)]
    pub class: ClassList,

    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Content displayed by the web view.
    #[props(start)]
    pub source: WebViewSource,

    /// Whether the web view uses a transparent page backing.
    #[props(default)]
    pub transparent: bool,

    /// Whether users can inspect the web view with platform developer tools.
    #[props(default)]
    pub inspectable: bool,

    /// Controller for imperative operations on the mounted web view.
    #[props(default)]
    #[props(inspect(skip))]
    pub controller: WebViewController,

    /// Optional native bridge installed before loading `source`.
    #[doc(hidden)]
    pub bridge: Option<Rc<dyn WebViewBridge>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn presenter(calls: Rc<Cell<usize>>) -> WebViewPresenter {
        WebViewPresenter {
            open_dev_tools: Shared::from(Rc::new(move || {
                calls.set(calls.get() + 1);
                Ok(())
            })
                as Rc<dyn Fn() -> Result<(), WebViewDevToolsError>>),
        }
    }

    #[test]
    fn controller_reports_when_it_is_not_mounted() {
        assert_eq!(
            WebViewController::new().open_dev_tools(),
            Err(WebViewDevToolsError::NotMounted)
        );
    }

    #[test]
    fn registration_connects_and_disconnects_the_controller() {
        let controller = WebViewController::new();
        let calls = Rc::new(Cell::new(0));
        let registration = controller.bind(presenter(calls.clone()));

        assert_eq!(controller.open_dev_tools(), Ok(()));
        assert_eq!(calls.get(), 1);

        drop(registration);
        assert_eq!(
            controller.open_dev_tools(),
            Err(WebViewDevToolsError::NotMounted)
        );
    }

    #[test]
    fn stale_registration_does_not_disconnect_a_new_binding() {
        let controller = WebViewController::new();
        let first = controller.bind(presenter(Rc::new(Cell::new(0))));
        let second_calls = Rc::new(Cell::new(0));
        let _second = controller.bind(presenter(second_calls.clone()));

        drop(first);
        assert_eq!(controller.open_dev_tools(), Ok(()));
        assert_eq!(second_calls.get(), 1);
    }
}
