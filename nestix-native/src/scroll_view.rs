pub use nestix_native_core::ScrollViewProps;

delegate!(
    /// Displays content in a viewport that can scroll on either axis.
    pub ScrollView(ScrollViewProps) => create_scroll_view
);
