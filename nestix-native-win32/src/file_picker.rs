use std::{cell::RefCell, ffi::c_void, path::PathBuf, rc::Rc};

use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    FilePickerCallback, FilePickerError, FilePickerMode, FilePickerOpenError, FilePickerOutcome,
    FilePickerPresenter, FilePickerProps, FilePickerRegistration, FilePickerRequest,
    FilePickerResult,
};
use windows::{
    Win32::{
        Foundation::ERROR_CANCELLED,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
        UI::Shell::{
            Common::COMDLG_FILTERSPEC, FILEOPENDIALOGOPTIONS, FOS_ALLOWMULTISELECT,
            FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST,
            FOS_PICKFOLDERS, FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog,
            IFileSaveDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
        },
    },
    core::{HRESULT, HSTRING, PCWSTR},
};

use crate::{root::ensure_com_apartment, window::WindowContext};

#[component]
pub fn FilePicker(props: &FilePickerProps, element: &Element) {
    let window = element.context::<WindowContext>().unwrap();
    let registration = Rc::new(RefCell::new(None::<FilePickerRegistration>));
    scoped_effect!(
        [props.controller, registration, window.hwnd] || {
            registration.borrow_mut().take();
            registration
                .borrow_mut()
                .replace(controller.get().bind(FilePickerPresenter {
                    open: callback!(
                        [hwnd] | request,
                        on_complete | present(hwnd, request, on_complete)
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
    owner: windows::Win32::Foundation::HWND,
    request: FilePickerRequest,
    on_complete: FilePickerCallback,
) -> Result<(), FilePickerOpenError> {
    ensure_com_apartment().map_err(FilePickerOpenError::BackendUnavailable)?;

    // IFileDialog::Show is modal and pumps the UI thread. The controller has
    // already marked the request busy, so synchronous completion is supported.
    let result = unsafe {
        match request.mode {
            FilePickerMode::SaveFile => {
                let dialog: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(backend_open_error)?;
                configure(&dialog, &request)?;
                dialog
                    .SetOptions(
                        dialog.GetOptions().map_err(backend_open_error)?
                            | mode_options(request.mode),
                    )
                    .map_err(backend_open_error)?;
                match show(&dialog, owner) {
                    Ok(true) => item_result(dialog.GetResult()),
                    Ok(false) => Ok(FilePickerOutcome::Cancelled),
                    Err(error) => Err(error),
                }
            }
            mode => {
                let dialog: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(backend_open_error)?;
                configure(&dialog, &request)?;
                dialog
                    .SetOptions(
                        dialog.GetOptions().map_err(backend_open_error)? | mode_options(mode),
                    )
                    .map_err(backend_open_error)?;
                match show(&dialog, owner) {
                    Ok(true) => open_results(&dialog, mode),
                    Ok(false) => Ok(FilePickerOutcome::Cancelled),
                    Err(error) => Err(error),
                }
            }
        }
    };
    on_complete(result);
    Ok(())
}

fn mode_options(mode: FilePickerMode) -> FILEOPENDIALOGOPTIONS {
    let mut options = FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
    match mode {
        FilePickerMode::OpenFile => options |= FOS_FILEMUSTEXIST,
        FilePickerMode::OpenFiles => options |= FOS_FILEMUSTEXIST | FOS_ALLOWMULTISELECT,
        FilePickerMode::SaveFile => options |= FOS_OVERWRITEPROMPT,
        FilePickerMode::SelectFolder => options |= FOS_PICKFOLDERS,
    }
    options
}

unsafe fn configure(
    dialog: &IFileDialog,
    request: &FilePickerRequest,
) -> Result<(), FilePickerOpenError> {
    if let Some(title) = &request.title {
        unsafe {
            dialog
                .SetTitle(&HSTRING::from(title))
                .map_err(backend_open_error)?
        };
    }
    if let Some(name) = &request.suggested_name {
        unsafe {
            dialog
                .SetFileName(&HSTRING::from(name))
                .map_err(backend_open_error)?
        };
    }
    if let Some(directory) = &request.initial_directory {
        let path = HSTRING::from(directory.as_os_str());
        let folder: IShellItem =
            unsafe { SHCreateItemFromParsingName(&path, None).map_err(backend_open_error)? };
        unsafe { dialog.SetFolder(&folder).map_err(backend_open_error)? };
    }

    let names = request
        .filters
        .iter()
        .map(|filter| HSTRING::from(&filter.name))
        .collect::<Vec<_>>();
    let specs = request
        .filters
        .iter()
        .map(|filter| {
            if filter.is_all_files() {
                HSTRING::from("*.*")
            } else {
                HSTRING::from(
                    filter
                        .extensions
                        .iter()
                        .map(|ext| format!("*.{ext}"))
                        .collect::<Vec<_>>()
                        .join(";"),
                )
            }
        })
        .collect::<Vec<_>>();
    let native = names
        .iter()
        .zip(&specs)
        .map(|(name, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect::<Vec<_>>();
    if !native.is_empty() {
        unsafe { dialog.SetFileTypes(&native).map_err(backend_open_error)? };
    }
    Ok(())
}

unsafe fn show(
    dialog: &IFileDialog,
    owner: windows::Win32::Foundation::HWND,
) -> Result<bool, FilePickerError> {
    match unsafe { dialog.Show(Some(owner)) } {
        Ok(()) => Ok(true),
        Err(error) if is_cancelled(error.code()) => Ok(false),
        Err(error) => Err(FilePickerError::Backend(format!(
            "Win32 file dialog failed: {error}"
        ))),
    }
}

unsafe fn open_results(dialog: &IFileOpenDialog, mode: FilePickerMode) -> FilePickerResult {
    if mode == FilePickerMode::OpenFiles {
        let items = unsafe { dialog.GetResults() }.map_err(backend_error)?;
        let count = unsafe { items.GetCount() }.map_err(backend_error)?;
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            paths.push(unsafe { shell_path(items.GetItemAt(index).map_err(backend_error)?) }?);
        }
        selected(paths)
    } else {
        item_result(unsafe { dialog.GetResult() })
    }
}

fn item_result(item: windows::core::Result<IShellItem>) -> FilePickerResult {
    selected(vec![unsafe { shell_path(item.map_err(backend_error)?) }?])
}

unsafe fn shell_path(item: IShellItem) -> Result<PathBuf, FilePickerError> {
    let raw = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
        .map_err(|_| FilePickerError::NonFilesystemSelection)?;
    let value = unsafe { raw.to_string() }.map_err(|_| FilePickerError::NonFilesystemSelection);
    unsafe { CoTaskMemFree(Some(raw.0.cast::<c_void>())) };
    value.map(PathBuf::from)
}

fn selected(paths: Vec<PathBuf>) -> FilePickerResult {
    if paths.is_empty() {
        Ok(FilePickerOutcome::Cancelled)
    } else {
        Ok(FilePickerOutcome::Selected(paths))
    }
}

fn is_cancelled(hr: HRESULT) -> bool {
    hr.0 as u32 == (0x8007_0000u32 | ERROR_CANCELLED.0)
}

fn backend_error(error: windows::core::Error) -> FilePickerError {
    FilePickerError::Backend(format!("Win32 file dialog failed: {error}"))
}

fn backend_open_error(error: windows::core::Error) -> FilePickerOpenError {
    FilePickerOpenError::BackendUnavailable(format!("Win32 file dialog unavailable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_flags_are_mapped() {
        assert_ne!(
            (mode_options(FilePickerMode::OpenFiles) & FOS_ALLOWMULTISELECT).0,
            0
        );
        assert_ne!(
            (mode_options(FilePickerMode::SelectFolder) & FOS_PICKFOLDERS).0,
            0
        );
        assert_eq!(
            (mode_options(FilePickerMode::OpenFile) & FOS_ALLOWMULTISELECT).0,
            0
        );
        assert_ne!(
            (mode_options(FilePickerMode::SaveFile) & FOS_OVERWRITEPROMPT).0,
            0
        );
    }

    #[test]
    fn maps_native_cancellation() {
        assert!(is_cancelled(HRESULT(
            (0x8007_0000u32 | ERROR_CANCELLED.0) as i32
        )));
    }
}
