use dpi::{LogicalUnit, PhysicalUnit, PixelUnit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Auto,
    Px(PixelUnit),
}

impl From<f64> for Length {
    fn from(value: f64) -> Self {
        Self::Px(PixelUnit::Logical(LogicalUnit::new(value)))
    }
}

impl From<i32> for Length {
    fn from(value: i32) -> Self {
        Self::Px(PixelUnit::Logical(LogicalUnit::new(value.into())))
    }
}

impl From<PhysicalUnit<i32>> for Length {
    fn from(value: PhysicalUnit<i32>) -> Self {
        Self::Px(PixelUnit::Physical(value))
    }
}
