use dpi::{LogicalUnit, PixelUnit};

/// A size or position that is either automatic or an explicit pixel length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    /// Lets the layout engine determine the value.
    Auto,
    /// Uses an explicit logical or physical pixel length.
    Length(PixelUnit),
}

impl Dimension {
    /// Returns whether this dimension is [`Dimension::Auto`].
    pub fn is_auto(&self) -> bool {
        matches!(self, Dimension::Auto)
    }
}

impl From<f64> for Dimension {
    fn from(value: f64) -> Self {
        Self::Length(PixelUnit::Logical(LogicalUnit::new(value)))
    }
}

impl From<i32> for Dimension {
    fn from(value: i32) -> Self {
        Self::Length(PixelUnit::Logical(LogicalUnit::new(value.into())))
    }
}

#[cfg(feature = "taffy")]
impl Dimension {
    /// Converts this value to its Taffy layout representation.
    pub fn to_taffy(&self, scale_factor: f64) -> taffy::Dimension {
        use taffy::prelude::FromLength;

        match self {
            Dimension::Auto => taffy::Dimension::auto(),
            Dimension::Length(pixel_unit) => match pixel_unit {
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
