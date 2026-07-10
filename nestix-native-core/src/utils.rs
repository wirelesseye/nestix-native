#[cfg(feature = "taffy")]
mod taffy {
    use crate::{Dimension, Rect};
    use taffy::style_helpers::FromLength;

    pub fn margin_to_taffy(
        margin: Rect<Dimension>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: dimension_to_length_percentage_auto(margin.top, scale_factor),
            bottom: dimension_to_length_percentage_auto(margin.bottom, scale_factor),
            left: dimension_to_length_percentage_auto(margin.left, scale_factor),
            right: dimension_to_length_percentage_auto(margin.right, scale_factor),
        }
    }

    pub fn padding_to_taffy(
        padding: Rect<Dimension>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentage> {
        taffy::Rect {
            top: dimension_to_length_percentage(padding.top, scale_factor),
            bottom: dimension_to_length_percentage(padding.bottom, scale_factor),
            left: dimension_to_length_percentage(padding.left, scale_factor),
            right: dimension_to_length_percentage(padding.right, scale_factor),
        }
    }

    pub fn gap_to_taffy(dimension: Dimension, scale_factor: f64) -> taffy::LengthPercentage {
        dimension_to_length_percentage(dimension, scale_factor)
    }

    pub fn inset_to_taffy(
        left: Dimension,
        top: Dimension,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            left: dimension_to_length_percentage_auto(left, scale_factor),
            top: dimension_to_length_percentage_auto(top, scale_factor),
            right: taffy::LengthPercentageAuto::auto(),
            bottom: taffy::LengthPercentageAuto::auto(),
        }
    }

    pub fn dimension_to_length_percentage_auto(
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

    pub fn dimension_to_length_percentage(
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
