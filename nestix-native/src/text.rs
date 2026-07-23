pub use nestix_native_core::TextProps;

delegate!(
    /// Displays read-only text using the configured font and style.
    pub Text(TextProps) => create_text
);
