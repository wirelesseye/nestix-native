#![allow(dead_code, non_snake_case)]

use nestix::{Element, callback, component, layout};
use nestix_native::{
    Button, FilePicker, FilePickerController, FilePickerFilter, FilePickerOutcome,
    FilePickerRequest, FlexView, Window,
};

#[component]
fn PickerControls() -> Element {
    let picker = FilePickerController::new();
    layout! {
        FlexView {
            FilePicker(.controller = picker.clone())
            Button(
                .title = "Open",
                .on_click = callback!(
                    [picker] || {
                        let _ = picker.open(
                            FilePickerRequest::open_file()
                                .with_filter(FilePickerFilter::new("Images", ["png", "jpg"])),
                            callback!(|_result| {}),
                        );
                    }
                ),
            )
        }
    }
}

#[test]
fn file_picker_compiles_through_layout() {
    let _window = layout! {
        Window {
            PickerControls
        }
    };

    let _ = FilePickerRequest::open_files();
    let _ = FilePickerRequest::save_file().with_suggested_name("document.txt");
    let _ = FilePickerRequest::select_folder();
    let _ = FilePickerOutcome::Cancelled;
}
