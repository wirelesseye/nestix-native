use std::{fmt::Debug, path::PathBuf, rc::Rc};

use nestix::{Layout, props};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Optional native bridge installed before loading `source`.
    #[doc(hidden)]
    pub bridge: Option<Rc<dyn WebViewBridge>>,

    /// Logical children owned by bridge-managed content.
    #[doc(hidden)]
    #[props(default)]
    pub children: Layout,
}
