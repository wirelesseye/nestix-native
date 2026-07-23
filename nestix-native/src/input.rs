pub use nestix_native_core::InputProps;

delegate!(
    /// Displays a single-line text input with reactive value updates.
    pub Input(InputProps) => create_input
);
