use std::{fmt, path::PathBuf, rc::Rc, sync::Arc};

use nestix::{Element, Shared, props};

/// A logical representation advertised by a native drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragDataType {
    /// A list of filesystem paths.
    Files,
    /// Encoded image data.
    Image,
    /// Plain text.
    Text,
}

/// A set of logical drag representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragDataTypes(u8);

impl DragDataTypes {
    /// No data representations.
    pub const NONE: Self = Self(0);
    /// Filesystem paths.
    pub const FILES: Self = Self(1 << 0);
    /// Encoded image data.
    pub const IMAGE: Self = Self(1 << 1);
    /// Plain text.
    pub const TEXT: Self = Self(1 << 2);
    /// Every supported representation.
    pub const ALL: Self = Self(Self::FILES.0 | Self::IMAGE.0 | Self::TEXT.0);

    /// Returns whether all flags in `value` are present.
    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }

    /// Returns whether the given logical representation is present.
    pub const fn contains_type(self, value: DragDataType) -> bool {
        self.contains(Self::from_type(value))
    }

    /// Creates a one-bit set for a logical representation.
    pub const fn from_type(value: DragDataType) -> Self {
        match value {
            DragDataType::Files => Self::FILES,
            DragDataType::Image => Self::IMAGE,
            DragDataType::Text => Self::TEXT,
        }
    }

    /// Returns the representations shared by both sets.
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Returns whether the set contains no representations.
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
    /// Copy the dragged data.
    Copy,
    /// Move the dragged data.
    Move,
    /// Create a link to the dragged data.
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
    /// Encoded image bytes.
    pub bytes: Arc<[u8]>,
    /// Media type describing the encoding, such as `image/png`.
    pub media_type: String,
    /// Preferred file name when materializing the image.
    pub suggested_name: String,
}

impl DragImage {
    /// Creates encoded drag image data.
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
    /// Data representations offered by the source.
    pub available_types: DragDataTypes,
    /// Operations permitted by the source.
    pub allowed_operations: DragOperations,
    /// Pointer position relative to the target.
    pub position: dpi::LogicalPosition<f64>,
    /// Keyboard modifiers active for the event.
    pub modifiers: DragModifiers,
}

#[derive(Debug, Clone)]
pub struct DropEvent {
    /// Operation negotiated for the drop.
    pub operation: DragOperation,
    /// Drop position relative to the target.
    pub position: dpi::LogicalPosition<f64>,
    /// Keyboard modifiers active for the drop.
    pub modifiers: DragModifiers,
    /// Reader used to request the dropped data.
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

/// Properties that make a visual element a native drag source.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DragSourceProps {
    /// Visual element that acts as the draggable target.
    pub children: Element,
    /// Data advertised by the drag.
    #[props(default)]
    pub content: DragContent,
    /// Whether dragging may begin.
    #[props(default = true)]
    pub enabled: bool,
    /// Operations the source permits a target to negotiate.
    #[props(default)]
    pub allowed_operations: DragOperations,
    /// Called after the native drag session starts.
    pub on_started: Option<Shared<dyn Fn()>>,
    /// Called when the drag session completes.
    pub on_completed: Option<Shared<dyn Fn(DragSourceOutcome)>>,
    /// Called when a drag session cannot be started.
    pub on_error: Option<Shared<dyn Fn(DragSourceError)>>,
}

/// Properties that make a visual element accept native drops.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct DropTargetProps {
    /// Visual element that defines the drop target bounds.
    pub children: Element,
    /// Whether the target accepts drops.
    #[props(default = true)]
    pub enabled: bool,
    /// Data representations accepted by the target.
    #[props(default)]
    pub accepted_types: DragDataTypes,
    /// Operation proposed when a callback does not override it.
    #[props(default = DragOperation::Copy)]
    pub default_operation: DragOperation,
    /// Called when a compatible drag enters the target.
    pub on_enter: Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>,
    /// Called as a compatible drag moves over the target.
    pub on_over: Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>,
    /// Called when a drag leaves the target without dropping.
    pub on_leave: Option<Shared<dyn Fn()>>,
    /// Called when data is dropped on the target.
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
