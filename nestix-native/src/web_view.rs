pub use nestix_native_core::WebViewProps;

delegate!(
    /// Displays web content loaded from a URL.
    pub WebView(WebViewProps) => create_web_view
);
