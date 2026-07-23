pub use nestix_native_core::FilePickerProps;

delegate!(
    /// Mounts a platform file-picker service controlled through [`FilePickerProps`].
    pub FilePicker(FilePickerProps) => create_file_picker
);
