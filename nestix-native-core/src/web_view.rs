use std::{fmt::Debug, path::PathBuf, rc::Rc};

use nestix::{Layout, props};

use crate::{ClassList, ViewProps};

/// Function supplied by a native web view for evaluating JavaScript.
pub type JavaScriptEvaluator = Rc<dyn Fn(&str)>;

/// Content source used by a managed web-view document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebViewDocumentSource {
    Html {
        html: String,
        base_url: Option<String>,
    },
    Resource {
        path: PathBuf,
        development_path: Option<PathBuf>,
    },
}

/// A managed document hosted by a native web view.
///
/// This keeps platform backends independent of the document implementation.
pub trait WebViewDocument: Debug {
    /// Content loaded into the web view.
    fn source(&self) -> WebViewDocumentSource;

    /// Script installed before the document loads.
    fn initialization_script(&self) -> Option<&str> {
        None
    }

    /// Name used by the document's script-message channel.
    fn message_handler_name(&self) -> &str;

    /// Connects the document to the web view's JavaScript evaluator.
    fn attach(&self, evaluate_javascript: JavaScriptEvaluator);

    /// Delivers a script message emitted by the document.
    fn receive_message(&self, message: &str);

    /// Disconnects the document from its web view.
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

    /// URL displayed when no managed document is supplied.
    #[props(start)]
    pub url: String,

    /// Whether the web view uses a transparent page backing.
    #[props(default)]
    pub transparent: bool,

    /// Managed document hosted instead of loading `url`.
    #[doc(hidden)]
    pub document: Option<Rc<dyn WebViewDocument>>,

    /// Logical children owned by a managed document.
    #[doc(hidden)]
    #[props(default)]
    pub children: Layout,
}
