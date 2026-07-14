/// Controls whether a component uses the default theme provided by its backend.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    /// Always use the backend's default theme.
    #[default]
    Native,
    /// Disable the backend's default theme so custom styles can take effect.
    None,
    /// Use the backend's default theme unless an incompatible custom style is set.
    Auto,
}
