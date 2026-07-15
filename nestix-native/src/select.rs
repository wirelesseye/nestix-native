pub use nestix_native_core::{SelectOptionProps, SelectProps};

delegate!(
    pub Select(SelectProps) => create_select,
    pub SelectOption(SelectOptionProps) => create_select_option,
);
