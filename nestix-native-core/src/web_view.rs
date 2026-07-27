use std::{fmt::Debug, path::PathBuf, rc::Rc};

use nestix::{Layout, props};

use crate::{ClassList, ViewProps};

/// Function supplied by a native web view for evaluating JavaScript.
pub type JavaScriptEvaluator = Rc<dyn Fn(&str)>;

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
    /// Script installed before the document loads.
    fn initialization_script(&self) -> Option<&str> {
        None
    }

    /// Name used by the document's script-message channel.
    fn message_handler_name(&self) -> &str;

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
