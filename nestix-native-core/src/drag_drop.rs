use std::{fmt, path::PathBuf, rc::Rc, sync::Arc};

use nestix::{Element, Shared, props};

/// A logical representation advertised by a native drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragDataType {
    Files,
    Image,
    Text,
}

/// A set of logical drag representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragDataTypes(u8);

impl DragDataTypes {
    pub const NONE: Self = Self(0);
    pub const FILES: Self = Self(1 << 0);
    pub const IMAGE: Self = Self(1 << 1);
    pub const TEXT: Self = Self(1 << 2);
    pub const ALL: Self = Self(Self::FILES.0 | Self::IMAGE.0 | Self::TEXT.0);

    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }

    pub const fn contains_type(self, value: DragDataType) -> bool {
        self.contains(Self::from_type(value))
    }

    pub const fn from_type(value: DragDataType) -> Self {
        match value {
            DragDataType::Files => Self::FILES,
            DragDataType::Image => Self::IMAGE,
            DragDataType::Text => Self::TEXT,
        }
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl Default for DragDataTypes {
    fn default() -> Self {
        Self::ALL
    }
}

impl std::ops::BitOr for DragDataTypes {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DragDataTypes {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// A drag operation negotiated between the source and target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragOperation {
    Copy,
    Move,
    Link,
}

/// A set of operations supported by a drag source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragOperations(u8);

impl DragOperations {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1 << 0);
    pub const MOVE: Self = Self(1 << 1);
    pub const LINK: Self = Self(1 << 2);
    pub const ALL: Self = Self(Self::COPY.0 | Self::MOVE.0 | Self::LINK.0);

    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }

    pub const fn contains_operation(self, value: DragOperation) -> bool {
        self.contains(Self::from_operation(value))
    }

    pub const fn from_operation(value: DragOperation) -> Self {
        match value {
            DragOperation::Copy => Self::COPY,
            DragOperation::Move => Self::MOVE,
            DragOperation::Link => Self::LINK,
        }
    }
}

impl Default for DragOperations {
    fn default() -> Self {
        Self::COPY
    }
}

impl std::ops::BitOr for DragOperations {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DragOperations {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Platform-neutral keyboard modifiers active during a drag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DragModifiers(u8);

impl DragModifiers {
    pub const NONE: Self = Self(0);
    /// Command on macOS and Control on Windows.
    pub const PRIMARY: Self = Self(1 << 0);
    pub const SHIFT: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);

    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }
}

impl std::ops::BitOr for DragModifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DragModifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Encoded image bytes advertised in a drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragImage {
    pub bytes: Arc<[u8]>,
    pub media_type: String,
    pub suggested_name: String,
}

impl DragImage {
    pub fn new(
        bytes: impl Into<Arc<[u8]>>,
        media_type: impl Into<String>,
        suggested_name: impl Into<String>,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            media_type: media_type.into(),
            suggested_name: suggested_name.into(),
        }
    }
}

/// Eager data supplied by a Nestix drag source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DragContent {
    files: Option<Vec<PathBuf>>,
    image: Option<DragImage>,
    text: Option<String>,
}

impl DragContent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_files(mut self, files: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.files = Some(files.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_image(mut self, image: DragImage) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn files(&self) -> Option<&[PathBuf]> {
        self.files.as_deref()
    }

    pub fn image(&self) -> Option<&DragImage> {
        self.image.as_ref()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn available_types(&self) -> DragDataTypes {
        let mut types = DragDataTypes::NONE;
        if self.files.as_ref().is_some_and(|files| !files.is_empty()) {
            types |= DragDataTypes::FILES;
        }
        if self.image.is_some() {
            types |= DragDataTypes::IMAGE;
        }
        if self.text.is_some() {
            types |= DragDataTypes::TEXT;
        }
        types
    }

    pub fn is_empty(&self) -> bool {
        self.available_types().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragReadError {
    Unavailable(DragDataType),
    InvalidData(String),
    Backend(String),
}

impl fmt::Display for DragReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(kind) => write!(formatter, "drag data {kind:?} is unavailable"),
            Self::InvalidData(message) => write!(formatter, "invalid drag data: {message}"),
            Self::Backend(message) => write!(formatter, "drag data backend failed: {message}"),
        }
    }
}

impl std::error::Error for DragReadError {}

pub type DragFilesCallback = Shared<dyn Fn(Result<Vec<PathBuf>, DragReadError>)>;
pub type DragImageCallback = Shared<dyn Fn(Result<DragImage, DragReadError>)>;
pub type DragTextCallback = Shared<dyn Fn(Result<String, DragReadError>)>;

/// Backend-provided lazy access to the representations in one completed drop.
#[doc(hidden)]
#[derive(Clone)]
pub struct DropDataProvider {
    pub available_types: DragDataTypes,
    pub read_files: Shared<dyn Fn(DragFilesCallback)>,
    pub read_image: Shared<dyn Fn(DragImageCallback)>,
    pub read_text: Shared<dyn Fn(DragTextCallback)>,
}

/// Cloneable lazy reader retained by a completed drop event.
#[derive(Clone)]
pub struct DropDataReader {
    provider: Rc<DropDataProvider>,
}

impl fmt::Debug for DropDataReader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DropDataReader")
            .field("available_types", &self.available_types())
            .finish()
    }
}

impl DropDataReader {
    #[doc(hidden)]
    pub fn new(provider: DropDataProvider) -> Self {
        Self {
            provider: Rc::new(provider),
        }
    }

    pub fn available_types(&self) -> DragDataTypes {
        self.provider.available_types
    }

    pub fn read_files(&self, callback: DragFilesCallback) {
        if self.available_types().contains(DragDataTypes::FILES) {
            (self.provider.read_files)(callback);
        } else {
            callback(Err(DragReadError::Unavailable(DragDataType::Files)));
        }
    }

    pub fn read_image(&self, callback: DragImageCallback) {
        if self.available_types().contains(DragDataTypes::IMAGE) {
            (self.provider.read_image)(callback);
        } else {
            callback(Err(DragReadError::Unavailable(DragDataType::Image)));
        }
    }

    pub fn read_text(&self, callback: DragTextCallback) {
        if self.available_types().contains(DragDataTypes::TEXT) {
            (self.provider.read_text)(callback);
        } else {
            callback(Err(DragReadError::Unavailable(DragDataType::Text)));
        }
    }
}

#[derive(Debug, Clone)]
pub struct DragOffer {
    pub available_types: DragDataTypes,
    pub allowed_operations: DragOperations,
    pub position: dpi::LogicalPosition<f64>,
    pub modifiers: DragModifiers,
}

#[derive(Debug, Clone)]
pub struct DropEvent {
    pub operation: DragOperation,
    pub position: dpi::LogicalPosition<f64>,
    pub modifiers: DragModifiers,
    pub data: DropDataReader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragSourceOutcome {
    Dropped(DragOperation),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragSourceError {
    EmptyContent,
    InvalidContent(String),
    Backend(String),
}

impl fmt::Display for DragSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContent => formatter.write_str("drag source has no data"),
            Self::InvalidContent(message) => write!(formatter, "invalid drag content: {message}"),
            Self::Backend(message) => write!(formatter, "drag source backend failed: {message}"),
        }
    }
}

impl std::error::Error for DragSourceError {}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct DragSourceProps {
    pub children: Element,
    #[props(default)]
    pub content: DragContent,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default)]
    pub allowed_operations: DragOperations,
    pub on_started: Option<Shared<dyn Fn()>>,
    pub on_completed: Option<Shared<dyn Fn(DragSourceOutcome)>>,
    pub on_error: Option<Shared<dyn Fn(DragSourceError)>>,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct DropTargetProps {
    pub children: Element,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default)]
    pub accepted_types: DragDataTypes,
    #[props(default = DragOperation::Copy)]
    pub default_operation: DragOperation,
    pub on_enter: Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>,
    pub on_over: Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>,
    pub on_leave: Option<Shared<dyn Fn()>>,
    pub on_drop: Shared<dyn Fn(DropEvent)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn content_advertises_only_non_empty_representations() {
        let content = DragContent::new()
            .with_files([PathBuf::from("a.txt")])
            .with_text("hello")
            .with_image(DragImage::new(
                [1, 2, 3].as_slice(),
                "image/png",
                "image.png",
            ));
        assert_eq!(content.available_types(), DragDataTypes::ALL);
        assert!(!content.is_empty());
        assert!(
            DragContent::new()
                .with_files(Vec::<PathBuf>::new())
                .is_empty()
        );
    }

    #[test]
    fn unavailable_reads_fail_without_calling_backend() {
        let backend_called = Rc::new(Cell::new(false));
        let reader = DropDataReader::new(DropDataProvider {
            available_types: DragDataTypes::TEXT,
            read_files: Shared::from({
                let backend_called = backend_called.clone();
                Rc::new(move |_| backend_called.set(true)) as Rc<dyn Fn(DragFilesCallback)>
            }),
            read_image: Shared::from(Rc::new(|_| {}) as Rc<dyn Fn(DragImageCallback)>),
            read_text: Shared::from(Rc::new(|callback: DragTextCallback| {
                callback(Ok("hello".into()))
            }) as Rc<dyn Fn(DragTextCallback)>),
        });
        let failed = Rc::new(Cell::new(false));
        reader.read_files(Shared::from({
            let failed = failed.clone();
            Rc::new(move |result| failed.set(matches!(result, Err(DragReadError::Unavailable(_)))))
                as Rc<dyn Fn(Result<Vec<PathBuf>, DragReadError>)>
        }));
        assert!(failed.get());
        assert!(!backend_called.get());
    }
}
