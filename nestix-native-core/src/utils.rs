#[cfg(feature = "taffy")]
mod taffy {
    use crate::{Length, Rect, WithAuto};
    use taffy::style_helpers::FromLength;

    pub fn margin_to_taffy(
        margin: Rect<WithAuto<Length>>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: length_with_auto_to_length_percentage_auto(margin.top, scale_factor),
            bottom: length_with_auto_to_length_percentage_auto(margin.bottom, scale_factor),
            left: length_with_auto_to_length_percentage_auto(margin.left, scale_factor),
            right: length_with_auto_to_length_percentage_auto(margin.right, scale_factor),
        }
    }

    pub fn padding_to_taffy(
        padding: Rect<WithAuto<Length>>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentage> {
        taffy::Rect {
            top: length_with_auto_to_length_percentage(padding.top, scale_factor),
            bottom: length_with_auto_to_length_percentage(padding.bottom, scale_factor),
            left: length_with_auto_to_length_percentage(padding.left, scale_factor),
            right: length_with_auto_to_length_percentage(padding.right, scale_factor),
        }
    }

    pub fn gap_to_taffy(
        length_with_auto: WithAuto<Length>,
        scale_factor: f64,
    ) -> taffy::LengthPercentage {
        length_with_auto_to_length_percentage(length_with_auto, scale_factor)
    }

    pub fn inset_to_taffy(
        left: WithAuto<Length>,
        top: WithAuto<Length>,
        scale_factor: f64,
    ) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            left: length_with_auto_to_length_percentage_auto(left, scale_factor),
            top: length_with_auto_to_length_percentage_auto(top, scale_factor),
            right: taffy::LengthPercentageAuto::auto(),
            bottom: taffy::LengthPercentageAuto::auto(),
        }
    }

    pub fn length_with_auto_to_length_percentage_auto(
        length_with_auto: WithAuto<Length>,
        scale_factor: f64,
    ) -> taffy::LengthPercentageAuto {
        match length_with_auto {
            WithAuto::Auto => taffy::LengthPercentageAuto::auto(),
            WithAuto::Value(pixel_unit) => {
                taffy::LengthPercentageAuto::from_length(pixel_unit.to_logical::<f32>(scale_factor))
            }
        }
    }

    pub fn length_with_auto_to_length_percentage(
        length_with_auto: WithAuto<Length>,
        scale_factor: f64,
    ) -> taffy::LengthPercentage {
        match length_with_auto {
            WithAuto::Auto => taffy::LengthPercentage::length(0.0),
            WithAuto::Value(pixel_unit) => {
                taffy::LengthPercentage::from_length(pixel_unit.to_logical::<f32>(scale_factor))
            }
        }
    }
}

#[cfg(feature = "taffy")]
pub use taffy::*;
