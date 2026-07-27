use std::{fmt::Debug, rc::Rc};

use nestix::{Layout, props};

use crate::{ClassList, ViewProps};

/// Function supplied by a native web view for evaluating JavaScript.
pub type JavaScriptEvaluator = Rc<dyn Fn(&str)>;

/// A managed document hosted by a native web view.
///
/// This keeps platform backends independent of the document implementation.
pub trait WebViewDocument: Debug {
    /// Initial HTML loaded into the web view.
    fn html(&self) -> &str;

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
