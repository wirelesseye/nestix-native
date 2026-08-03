pub use nestix_native_core::{BorderProps, FlexViewProps};

delegate!(
    /// Arranges child components using a flex layout.
    pub FlexView(FlexViewProps) => create_flex_view
);
