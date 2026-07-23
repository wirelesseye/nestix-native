pub use nestix_native_core::ButtonProps;

delegate!(
    /// Displays a push button that invokes a callback when activated.
    pub Button(ButtonProps) => create_button
);
