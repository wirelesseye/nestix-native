pub use nestix_native_core::CheckboxProps;

delegate!(
    /// Displays a checkbox whose checked state can be controlled reactively.
    pub Checkbox(CheckboxProps) => create_checkbox
);
