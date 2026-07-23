pub use nestix_native_core::SliderProps;

delegate!(
    /// Displays a control for choosing a numeric value within a range.
    pub Slider(SliderProps) => create_slider
);
