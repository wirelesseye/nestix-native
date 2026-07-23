pub use nestix_native_core::WindowProps;

delegate!(
    /// Creates a top-level window for its child content.
    pub Window(WindowProps) => create_window
);
