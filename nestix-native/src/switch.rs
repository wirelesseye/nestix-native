pub use nestix_native_core::SwitchProps;

delegate!(
    /// Displays an on/off control with a reactive checked state.
    pub Switch(SwitchProps) => create_switch
);
