use dpi::{LogicalUnit, PhysicalUnit, Pixel, PixelUnit};

/// An explicit logical or physical pixel length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length(PixelUnit);

impl Length {
    pub fn logical(value: impl Into<f64>) -> Self {
        Self(PixelUnit::Logical(LogicalUnit::new(value.into())))
    }

    pub fn physical(value: impl Into<i32>) -> Self {
        Self(PixelUnit::Physical(PhysicalUnit::new(value.into())))
    }

    pub fn to_logical<P: Pixel>(&self, scale_factor: f64) -> LogicalUnit<P> {
        self.0.to_logical(scale_factor)
    }
}

impl From<PixelUnit> for Length {
    fn from(value: PixelUnit) -> Self {
        Self(value)
    }
}

impl From<f64> for Length {
    fn from(value: f64) -> Self {
        Self::logical(value)
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::logical(value)
    }
}

impl From<i32> for Length {
    fn from(value: i32) -> Self {
        Self::logical(value)
    }
}

/// A value that is either automatic or explicit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WithAuto<T> {
    /// Lets the layout engine determine the value.
    Auto,
    /// Uses an explicit value.
    Value(T),
}

impl<T> WithAuto<T> {
    /// Returns whether this value is [`WithAuto::Auto`].
    pub fn is_auto(&self) -> bool {
        matches!(self, WithAuto::Auto)
    }
}

impl<T> From<T> for WithAuto<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

impl From<f64> for WithAuto<Length> {
    fn from(value: f64) -> Self {
        Self::Value(Length::logical(value))
    }
}

impl From<f32> for WithAuto<Length> {
    fn from(value: f32) -> Self {
        Self::Value(Length::logical(value))
    }
}

impl From<i32> for WithAuto<Length> {
    fn from(value: i32) -> Self {
        Self::Value(Length::logical(value))
    }
}

#[cfg(feature = "taffy")]
impl WithAuto<Length> {
    /// Converts this value to its Taffy layout representation.
    pub fn to_taffy(&self, scale_factor: f64) -> taffy::Dimension {
        use taffy::prelude::FromLength;

        match self {
            WithAuto::Auto => taffy::Dimension::auto(),
            WithAuto::Value(Length(pixel_unit)) => match pixel_unit {
                PixelUnit::Physical(physical_unit) => {
                    taffy::Dimension::from_length(physical_unit.to_logical::<f32>(scale_factor))
                }
                PixelUnit::Logical(logical_unit) => taffy::Dimension::from_length(*logical_unit),
            },
        }
    }
}

/// Four edge values arranged as a rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect<T> {
    /// Top edge.
    pub top: T,
    /// Bottom edge.
    pub bottom: T,
    /// Left edge.
    pub left: T,
    /// Right edge.
    pub right: T,
}
