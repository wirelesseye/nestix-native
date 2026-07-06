#[cfg(feature = "taffy")]
mod taffy {
    use crate::{Dimension, Rect};
    use taffy::style_helpers::FromLength;

    pub fn margin_to_taffy(
        margin: Rect<Dimension>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: margin_dimension_to_taffy(margin.top, scale_factor),
            bottom: margin_dimension_to_taffy(margin.bottom, scale_factor),
            left: margin_dimension_to_taffy(margin.left, scale_factor),
            right: margin_dimension_to_taffy(margin.right, scale_factor),
        }
    }

    fn margin_dimension_to_taffy(
        dimension: Dimension,
        scale_factor: f64,
    ) -> taffy::LengthPercentageAuto {
        match dimension {
            Dimension::Auto => taffy::LengthPercentageAuto::auto(),
            Dimension::Length(pixel_unit) => {
                taffy::LengthPercentageAuto::from_length(pixel_unit.to_logical::<f32>(scale_factor))
            }
        }
    }

    pub fn padding_to_taffy(
        padding: Rect<Dimension>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentage> {
        taffy::Rect {
            top: padding_dimension_to_taffy(padding.top, scale_factor),
            bottom: padding_dimension_to_taffy(padding.bottom, scale_factor),
            left: padding_dimension_to_taffy(padding.left, scale_factor),
            right: padding_dimension_to_taffy(padding.right, scale_factor),
        }
    }

    fn padding_dimension_to_taffy(
        dimension: Dimension,
        scale_factor: f64,
    ) -> taffy::LengthPercentage {
        match dimension {
            Dimension::Auto => taffy::LengthPercentage::length(0.0),
            Dimension::Length(pixel_unit) => {
                taffy::LengthPercentage::from_length(pixel_unit.to_logical::<f32>(scale_factor))
            }
        }
    }
}

#[cfg(feature = "taffy")]
pub use taffy::*;
