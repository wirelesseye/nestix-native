use std::{cell::RefCell, fmt, path::PathBuf, rc::Rc};

use nestix::{Shared, props};

/// The native dialog operation requested from a [`FilePickerController`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FilePickerMode {
    #[default]
    OpenFile,
    OpenFiles,
    SaveFile,
    SelectFolder,
}

/// A labelled group of filename extensions shown by a file picker.
///
/// Extensions are written without a leading dot or wildcard, for example
/// `FilePickerFilter::new("Images", ["png", "jpg"])`. An empty extension
/// list represents all files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePickerFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FilePickerFilter {
    pub fn new(
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }

    pub fn all_files(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            extensions: Vec::new(),
        }
    }

    pub fn is_all_files(&self) -> bool {
        self.extensions.is_empty()
    }
}

/// Configuration for one native picker presentation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilePickerRequest {
    pub mode: FilePickerMode,
    pub title: Option<String>,
    pub initial_directory: Option<PathBuf>,
    pub suggested_name: Option<String>,
    pub filters: Vec<FilePickerFilter>,
}

impl FilePickerRequest {
    pub fn open_file() -> Self {
        Self::default()
    }

    pub fn open_files() -> Self {
        Self {
            mode: FilePickerMode::OpenFiles,
            ..Self::default()
        }
    }

    pub fn save_file() -> Self {
        Self {
            mode: FilePickerMode::SaveFile,
            ..Self::default()
        }
    }

    pub fn select_folder() -> Self {
        Self {
            mode: FilePickerMode::SelectFolder,
            ..Self::default()
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_initial_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.initial_directory = Some(directory.into());
        self
    }

    pub fn with_suggested_name(mut self, name: impl Into<String>) -> Self {
        self.suggested_name = Some(name.into());
        self
    }

    pub fn with_filter(mut self, filter: FilePickerFilter) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn with_filters(mut self, filters: impl IntoIterator<Item = FilePickerFilter>) -> Self {
        self.filters.extend(filters);
        self
    }

    fn validate(&self) -> Result<(), FilePickerOpenError> {
        for filter in &self.filters {
            for extension in &filter.extensions {
                if extension.is_empty()
                    || extension.starts_with('.')
                    || extension.contains('*')
                    || extension.contains('/')
                    || extension.contains('\\')
                {
                    return Err(FilePickerOpenError::InvalidRequest(format!(
                        "invalid file extension {extension:?}; use an extension without a dot or wildcard"
                    )));
                }
            }
        }
        Ok(())
    }
}

/// The user-visible outcome of a successfully presented picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePickerOutcome {
    Selected(Vec<PathBuf>),
    Cancelled,
}

/// Error produced after the backend accepted a picker request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePickerError {
    Backend(String),
    NonFilesystemSelection,
}

impl fmt::Display for FilePickerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(message) => write!(formatter, "file picker failed: {message}"),
            Self::NonFilesystemSelection => {
                formatter.write_str("the selected item does not have a filesystem path")
            }
        }
    }
}

impl std::error::Error for FilePickerError {}

pub type FilePickerResult = Result<FilePickerOutcome, FilePickerError>;
pub type FilePickerCallback = Shared<dyn Fn(FilePickerResult)>;

/// Error returned synchronously when a picker request cannot be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePickerOpenError {
    NotMounted,
    AlreadyOpen,
    InvalidRequest(String),
    BackendUnavailable(String),
}

impl fmt::Display for FilePickerOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted => formatter.write_str("file picker is not mounted"),
            Self::AlreadyOpen => formatter.write_str("file picker is already open"),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid file picker request: {message}")
            }
            Self::BackendUnavailable(message) => {
                write!(formatter, "file picker backend is unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for FilePickerOpenError {}

#[doc(hidden)]
#[derive(Clone)]
pub struct FilePickerPresenter {
    pub open:
        Shared<dyn Fn(FilePickerRequest, FilePickerCallback) -> Result<(), FilePickerOpenError>>,
}

#[derive(Default)]
struct FilePickerControllerState {
    next_binding_id: u64,
    next_request_id: u64,
    presenter: Option<(u64, FilePickerPresenter)>,
    pending: Option<(u64, u64)>,
}

/// Cloneable handle for opening a [`FilePickerProps`] mounted beneath a window.
#[derive(Clone, Default)]
pub struct FilePickerController {
    state: Rc<RefCell<FilePickerControllerState>>,
}

impl fmt::Debug for FilePickerController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        formatter
            .debug_struct("FilePickerController")
            .field("mounted", &state.presenter.is_some())
            .field("open", &state.pending.is_some())
            .finish()
    }
}

impl FilePickerController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.state.borrow().pending.is_some()
    }

    pub fn open(
        &self,
        request: FilePickerRequest,
        on_complete: FilePickerCallback,
    ) -> Result<(), FilePickerOpenError> {
        request.validate()?;

        let (binding_id, request_id, presenter) = {
            let mut state = self.state.borrow_mut();
            let (binding_id, presenter) = state
                .presenter
                .as_ref()
                .map(|(binding_id, presenter)| (*binding_id, presenter.clone()))
                .ok_or(FilePickerOpenError::NotMounted)?;
            if state.pending.is_some() {
                return Err(FilePickerOpenError::AlreadyOpen);
            }
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.wrapping_add(1);
            state.pending = Some((binding_id, request_id));
            (binding_id, request_id, presenter)
        };

        let controller = self.clone();
        let complete = Shared::from(Rc::new(move |result: FilePickerResult| {
            let should_complete = {
                let mut state = controller.state.borrow_mut();
                if state.pending == Some((binding_id, request_id)) {
                    state.pending = None;
                    true
                } else {
                    false
                }
            };
            if should_complete {
                on_complete(result);
            }
        }) as Rc<dyn Fn(FilePickerResult)>);

        if let Err(error) = (presenter.open)(request, complete) {
            let mut state = self.state.borrow_mut();
            if state.pending == Some((binding_id, request_id)) {
                state.pending = None;
            }
            return Err(error);
        }

        Ok(())
    }

    #[doc(hidden)]
    pub fn bind(&self, presenter: FilePickerPresenter) -> FilePickerRegistration {
        let mut state = self.state.borrow_mut();
        let binding_id = state.next_binding_id;
        state.next_binding_id = state.next_binding_id.wrapping_add(1);
        state.pending = None;
        state.presenter = Some((binding_id, presenter));
        FilePickerRegistration {
            controller: self.clone(),
            binding_id,
        }
    }
}

#[doc(hidden)]
pub struct FilePickerRegistration {
    controller: FilePickerController,
    binding_id: u64,
}

impl Drop for FilePickerRegistration {
    fn drop(&mut self) {
        let mut state = self.controller.state.borrow_mut();
        if state
            .presenter
            .as_ref()
            .is_some_and(|(binding_id, _)| *binding_id == self.binding_id)
        {
            state.presenter = None;
            if state
                .pending
                .is_some_and(|(binding_id, _)| binding_id == self.binding_id)
            {
                state.pending = None;
            }
        }
    }
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct FilePickerProps {
    pub controller: FilePickerController,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn callback(f: impl Fn(FilePickerResult) + 'static) -> FilePickerCallback {
        Shared::from(Rc::new(f) as Rc<dyn Fn(FilePickerResult)>)
    }

    #[test]
    fn request_builders_set_expected_modes_and_options() {
        let request = FilePickerRequest::save_file()
            .with_title("Export")
            .with_initial_directory("/tmp")
            .with_suggested_name("report.txt")
            .with_filter(FilePickerFilter::new("Text", ["txt"]));

        assert_eq!(request.mode, FilePickerMode::SaveFile);
        assert_eq!(request.title.as_deref(), Some("Export"));
        assert_eq!(request.initial_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(request.suggested_name.as_deref(), Some("report.txt"));
        assert_eq!(request.filters[0].extensions, ["txt"]);
        assert_eq!(
            FilePickerRequest::open_files().mode,
            FilePickerMode::OpenFiles
        );
        assert_eq!(
            FilePickerRequest::select_folder().mode,
            FilePickerMode::SelectFolder
        );
    }

    #[test]
    fn controller_rejects_unmounted_and_invalid_requests() {
        let controller = FilePickerController::new();
        assert_eq!(
            controller.open(FilePickerRequest::open_file(), callback(|_| {})),
            Err(FilePickerOpenError::NotMounted)
        );

        let _registration = controller.bind(FilePickerPresenter {
            open: Shared::from(Rc::new(|_, _| Ok(()))
                as Rc<
                    dyn Fn(
                        FilePickerRequest,
                        FilePickerCallback,
                    ) -> Result<(), FilePickerOpenError>,
                >),
        });
        let invalid =
            FilePickerRequest::open_file().with_filter(FilePickerFilter::new("Images", ["*.png"]));
        assert!(matches!(
            controller.open(invalid, callback(|_| {})),
            Err(FilePickerOpenError::InvalidRequest(_))
        ));
    }

    #[test]
    fn controller_tracks_completion_and_allows_callback_reentry() {
        let controller = FilePickerController::new();
        let completions = Rc::new(RefCell::new(Vec::<FilePickerCallback>::new()));
        let _registration = controller.bind(FilePickerPresenter {
            open: Shared::from({
                let completions = completions.clone();
                Rc::new(move |_, completion| {
                    completions.borrow_mut().push(completion);
                    Ok(())
                })
                    as Rc<
                        dyn Fn(
                            FilePickerRequest,
                            FilePickerCallback,
                        ) -> Result<(), FilePickerOpenError>,
                    >
            }),
        });

        controller
            .open(FilePickerRequest::open_file(), callback(|_| {}))
            .unwrap();
        assert!(controller.is_open());
        assert_eq!(
            controller.open(FilePickerRequest::open_file(), callback(|_| {})),
            Err(FilePickerOpenError::AlreadyOpen)
        );

        let reentered = Rc::new(Cell::new(false));
        let controller_for_callback = controller.clone();
        let reentered_for_callback = reentered.clone();
        completions.borrow()[0](Ok(FilePickerOutcome::Cancelled));
        assert!(!controller.is_open());
        controller
            .open(
                FilePickerRequest::open_file(),
                callback(move |_| {
                    reentered_for_callback.set(!controller_for_callback.is_open());
                }),
            )
            .unwrap();
        completions.borrow()[1](Ok(FilePickerOutcome::Cancelled));
        assert!(reentered.get());
    }

    #[test]
    fn stale_and_duplicate_completions_are_ignored() {
        let controller = FilePickerController::new();
        let completion = Rc::new(RefCell::new(None::<FilePickerCallback>));
        let registration = controller.bind(FilePickerPresenter {
            open: Shared::from({
                let completion = completion.clone();
                Rc::new(move |_, value| {
                    completion.replace(Some(value));
                    Ok(())
                })
                    as Rc<
                        dyn Fn(
                            FilePickerRequest,
                            FilePickerCallback,
                        ) -> Result<(), FilePickerOpenError>,
                    >
            }),
        });
        let calls = Rc::new(Cell::new(0));
        controller
            .open(
                FilePickerRequest::open_file(),
                callback({
                    let calls = calls.clone();
                    move |_| calls.set(calls.get() + 1)
                }),
            )
            .unwrap();
        let native_completion = completion.borrow().clone().unwrap();
        native_completion(Ok(FilePickerOutcome::Cancelled));
        native_completion(Ok(FilePickerOutcome::Cancelled));
        assert_eq!(calls.get(), 1);

        controller
            .open(FilePickerRequest::open_file(), callback(|_| {}))
            .unwrap();
        let stale_completion = completion.borrow().clone().unwrap();
        drop(registration);
        stale_completion(Ok(FilePickerOutcome::Cancelled));
        assert!(!controller.is_open());
    }
}
