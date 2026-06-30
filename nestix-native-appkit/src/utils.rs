use nestix_native_core::{Dimension, Rect};
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
        Dimension::Auto => taffy::LengthPercentageAuto::length(0.0),
        Dimension::Length(pixel_unit) => {
            taffy::LengthPercentageAuto::from_length(pixel_unit.to_logical::<f32>(scale_factor))
        }
    }
}
