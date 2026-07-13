use std::{path::PathBuf, sync::Arc};

use nestix::props;

use crate::{ClassList, ViewProps};

/// Encoded image data that can be decoded by the active native backend.
#[derive(Debug, Clone)]
pub enum ImageSource {
    File(PathBuf),
    Bytes(Arc<[u8]>),
}

impl ImageSource {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self::Bytes(bytes.into())
    }
}

impl From<PathBuf> for ImageSource {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<&[u8]> for ImageSource {
    fn from(bytes: &[u8]) -> Self {
        Self::Bytes(Arc::from(bytes))
    }
}

/// How an image is scaled inside the bounds of an [`ImageViewProps`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentFit {
    #[default]
    Contain,
    Cover,
    Fill,
    None,
    ScaleDown,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct ImageViewProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    /// The image to display. This is a named, required prop.
    pub source: ImageSource,

    #[props(default)]
    pub content_fit: ContentFit,
}
