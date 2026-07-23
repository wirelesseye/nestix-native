pub use nestix_native_core::{SelectOptionProps, SelectProps};

delegate!(
    /// Displays a control for selecting one value from child options.
    pub Select(SelectProps) => create_select,
    /// Defines one labelled value within a [`Select`].
    pub SelectOption(SelectOptionProps) => create_select_option,
);
