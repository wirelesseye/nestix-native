pub use nestix_native_core::WebViewProps;

delegate!(
    /// Displays content from a URL, inline HTML, or a packaged resource.
    pub WebView(WebViewProps) => create_web_view
);
