use std::{cell::RefCell, path::PathBuf, rc::Rc};

use block2::RcBlock;
use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    FilePickerCallback, FilePickerError, FilePickerMode, FilePickerOpenError, FilePickerOutcome,
    FilePickerPresenter, FilePickerProps, FilePickerRegistration, FilePickerRequest,
    FilePickerResult,
};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSModalResponseAbort, NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSSavePanel,
};
use objc2_foundation::{NSArray, NSString, NSURL};

use crate::window::WindowContext;

#[component]
pub fn FilePicker(props: &FilePickerProps, element: &Element) {
    let window = element.context::<WindowContext>().unwrap();
    let registration = Rc::new(RefCell::new(None::<FilePickerRegistration>));
    scoped_effect!(
        element,
        [props.controller, registration, window.ns_window] || {
            registration.borrow_mut().take();
            registration
                .borrow_mut()
                .replace(controller.get().bind(FilePickerPresenter {
                    open: callback!(
                        [ns_window] | request,
                        on_complete | { present(&ns_window, request, on_complete) }
                    ),
                }));
        }
    );

    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
}

fn present(
    window: &objc2_app_kit::NSWindow,
    request: FilePickerRequest,
    on_complete: FilePickerCallback,
) -> Result<(), FilePickerOpenError> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        FilePickerOpenError::BackendUnavailable(
            "AppKit file pickers must be opened on the main thread".to_string(),
        )
    })?;

    match request.mode {
        FilePickerMode::OpenFile | FilePickerMode::OpenFiles | FilePickerMode::SelectFolder => {
            let panel = NSOpenPanel::openPanel(mtm);
            panel.setCanChooseFiles(request.mode != FilePickerMode::SelectFolder);
            panel.setCanChooseDirectories(request.mode == FilePickerMode::SelectFolder);
            panel.setAllowsMultipleSelection(request.mode == FilePickerMode::OpenFiles);
            configure_panel(&panel, &request);

            let completion_panel = panel.clone();
            let handler = RcBlock::new(move |response| {
                let result = open_result(&completion_panel, response);
                on_complete(result);
            });
            panel.beginSheetModalForWindow_completionHandler(window, &handler);
        }
        FilePickerMode::SaveFile => {
            let panel = NSSavePanel::savePanel(mtm);
            configure_panel(&panel, &request);
            if let Some(name) = &request.suggested_name {
                panel.setNameFieldStringValue(&NSString::from_str(name));
            }

            let completion_panel = panel.clone();
            let handler = RcBlock::new(move |response| {
                let result = save_result(&completion_panel, response);
                on_complete(result);
            });
            panel.beginSheetModalForWindow_completionHandler(window, &handler);
        }
    }

    Ok(())
}

fn configure_panel(panel: &NSSavePanel, request: &FilePickerRequest) {
    if let Some(title) = &request.title {
        panel.setTitle(Some(&NSString::from_str(title)));
    }
    if let Some(directory) = &request.initial_directory {
        let directory = NSString::from_str(&directory.to_string_lossy());
        panel.setDirectoryURL(Some(&NSURL::fileURLWithPath_isDirectory(&directory, true)));
    }

    let extensions = request
        .filters
        .iter()
        .flat_map(|filter| filter.extensions.iter())
        .map(|extension| NSString::from_str(extension))
        .collect::<Vec<_>>();
    #[allow(deprecated)]
    if request.filters.iter().any(|filter| filter.is_all_files()) || extensions.is_empty() {
        panel.setAllowedFileTypes(None);
    } else {
        let extensions = NSArray::from_retained_slice(&extensions);
        panel.setAllowedFileTypes(Some(&extensions));
    }
}

fn open_result(panel: &NSOpenPanel, response: isize) -> FilePickerResult {
    if response == NSModalResponseOK {
        paths_result(panel.URLs().to_vec())
    } else if response == NSModalResponseCancel {
        Ok(FilePickerOutcome::Cancelled)
    } else if response == NSModalResponseAbort {
        Err(FilePickerError::Backend(
            "AppKit aborted file picker presentation".to_string(),
        ))
    } else {
        Err(FilePickerError::Backend(format!(
            "AppKit returned unexpected modal response {response}"
        )))
    }
}

fn save_result(panel: &NSSavePanel, response: isize) -> FilePickerResult {
    if response == NSModalResponseOK {
        match panel.URL() {
            Some(url) => paths_result(vec![url]),
            None => Err(FilePickerError::NonFilesystemSelection),
        }
    } else if response == NSModalResponseCancel {
        Ok(FilePickerOutcome::Cancelled)
    } else if response == NSModalResponseAbort {
        Err(FilePickerError::Backend(
            "AppKit aborted file picker presentation".to_string(),
        ))
    } else {
        Err(FilePickerError::Backend(format!(
            "AppKit returned unexpected modal response {response}"
        )))
    }
}

fn paths_result(urls: Vec<objc2::rc::Retained<NSURL>>) -> FilePickerResult {
    let paths = urls
        .into_iter()
        .map(|url| {
            if !url.isFileURL() {
                return Err(FilePickerError::NonFilesystemSelection);
            }
            url.path()
                .map(|path| PathBuf::from(path.to_string()))
                .ok_or(FilePickerError::NonFilesystemSelection)
        })
        .collect::<Result<Vec<_>, _>>()?;

    if paths.is_empty() {
        Ok(FilePickerOutcome::Cancelled)
    } else {
        Ok(FilePickerOutcome::Selected(paths))
    }
}
