use std::path::PathBuf;

use nestix::{Layout, props};

use crate::{ClassList, ViewProps, WebViewController};

/// HTML template used by a managed DOM surface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DomTemplate {
    /// Nestix's minimal built-in document.
    #[default]
    Default,

    /// HTML supplied directly by the application.
    Html {
        html: String,
        base_url: Option<String>,
    },

    /// A logical path beneath the packaged application's resource directory.
    Resource {
        path: PathBuf,
        development_path: Option<PathBuf>,
    },
}

impl DomTemplate {
    /// Creates an in-memory template without a base URL.
    pub fn html(html: impl Into<String>) -> Self {
        Self::Html {
            html: html.into(),
            base_url: None,
        }
    }

    /// Creates an in-memory template whose relative URLs use `base_url`.
    pub fn html_with_base_url(html: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::Html {
            html: html.into(),
            base_url: Some(base_url.into()),
        }
    }

    /// Creates a template from a logical packaged-resource path.
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
            template => template,
        }
    }
}

/// Properties for a managed DOM subtree embedded in a native view.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DomSurfaceProps {
    /// Style classes applied to the native surface.
    #[props(default)]
    pub class: ClassList,

    /// Common outer layout properties. The surface is one native layout leaf.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Whether the managed document has a transparent background.
    #[props(default = true)]
    pub transparent: bool,

    /// Whether users can inspect the managed document with platform developer tools.
    #[props(default)]
    pub inspectable: bool,

    /// Controller for imperative operations on the managed web view.
    #[props(default)]
    pub controller: WebViewController,

    /// Document template loaded before Nestix injects its managed DOM runtime.
    #[props(default)]
    pub template: DomTemplate,

    /// Components rendered into the managed DOM document.
    #[props(default)]
    pub children: Layout,
}
