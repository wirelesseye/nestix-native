use dpi::{LogicalUnit, PixelUnit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Length(PixelUnit),
}

impl Dimension {
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
    pub fn into_taffy_dimension(self, scale_factor: f64) -> taffy::Dimension {
        use taffy::prelude::FromLength;

        match self {
            Dimension::Auto => taffy::Dimension::auto(),
            Dimension::Length(pixel_unit) => match pixel_unit {
                PixelUnit::Physical(physical_unit) => {
                    taffy::Dimension::from_length(physical_unit.to_logical::<f32>(scale_factor))
                }
                PixelUnit::Logical(logical_unit) => taffy::Dimension::from_length(logical_unit),
            },
        }
    }
}
