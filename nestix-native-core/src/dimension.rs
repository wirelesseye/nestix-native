use dpi::{LogicalUnit, PhysicalUnit, Pixel};

/// An explicit logical, physical, or font-relative length.
#[derive(Debug, Clone, Copy, PartialEq, nestix::InspectableValue)]
pub enum Length {
    /// Device-independent logical pixels.
    Logical(f64),
    /// Device pixels, converted using the current scale factor.
    Physical(i32),
    /// A multiple of the element's computed font size.
    Em(f64),
}

impl Length {
    pub fn logical(value: impl Into<f64>) -> Self {
        Self::Logical(value.into())
    }

    pub fn physical(value: impl Into<i32>) -> Self {
        Self::Physical(value.into())
    }

    /// Creates a length relative to the element's computed font size.
    pub fn em(value: impl Into<f64>) -> Self {
        Self::Em(value.into())
    }

    /// Resolves font-relative units using `font_size`, preserving pixel units.
    pub fn resolve(self, font_size: f64) -> Self {
        match self {
            Self::Em(value) => Self::Logical(value * font_size),
            value => value,
        }
    }

    /// Resolves this length to logical pixels after font-relative units have been computed.
    ///
    /// # Panics
    ///
    /// Panics if called directly on [`Length::Em`]. Use [`Length::resolve`] first.
    pub fn to_logical<P: Pixel>(&self, scale_factor: f64) -> LogicalUnit<P> {
        match *self {
            Self::Logical(value) => LogicalUnit::new(value.cast()),
            Self::Physical(value) => PhysicalUnit::new(value).to_logical(scale_factor),
            Self::Em(_) => panic!("em length must be resolved against a computed font size"),
        }
    }

    /// Resolves this length to physical pixels after font-relative units have been computed.
    ///
    /// # Panics
    ///
    /// Panics if called directly on [`Length::Em`]. Use [`Length::resolve`] first.
    pub fn to_physical<P: Pixel>(&self, scale_factor: f64) -> PhysicalUnit<P> {
        match *self {
            Self::Logical(value) => LogicalUnit::new(value).to_physical(scale_factor),
            Self::Physical(value) => PhysicalUnit::new(value.cast()),
            Self::Em(_) => panic!("em length must be resolved against a computed font size"),
        }
    }
}

impl From<dpi::PixelUnit> for Length {
    fn from(value: dpi::PixelUnit) -> Self {
        match value {
            dpi::PixelUnit::Logical(value) => Self::Logical(value.0),
            dpi::PixelUnit::Physical(value) => Self::Physical(value.0),
        }
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
#[derive(Debug, Clone, Copy, PartialEq, nestix::InspectableValue)]
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
            WithAuto::Value(length) => {
                taffy::Dimension::from_length(length.to_logical::<f32>(scale_factor))
            }
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
