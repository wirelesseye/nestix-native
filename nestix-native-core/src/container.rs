use nestix::{Computed, PropValue, computed, props};

use crate::{Length, Rect, WithAuto};

/// Padding properties shared by container-like controls.
#[props(
    debug,
    default,
    group(padding => [padding_left, padding_right, padding_top, padding_bottom]),
    group(padding_horizontal => [padding_left, padding_right]),
    group(padding_vertical => [padding_top, padding_bottom]),
)]
#[derive(Debug, Clone)]
pub struct ContainerProps {
    /// Padding on the left edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub padding_left: WithAuto<Length>,
    /// Padding on the right edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub padding_right: WithAuto<Length>,
    /// Padding on the top edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub padding_top: WithAuto<Length>,
    /// Padding on the bottom edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub padding_bottom: WithAuto<Length>,
}

impl ContainerProps {
    pub(crate) fn auto_padding() -> Self {
        Self::builder()
            .padding(PropValue::from_plain(WithAuto::Auto))
            .build()
    }

    /// Returns the four reactive padding values as a rectangle.
    pub fn padding(&self) -> Computed<Rect<WithAuto<Length>>> {
        computed!([this: self] || {
            let top = this.padding_top.get();
            let bottom = this.padding_bottom.get();
            let left = this.padding_left.get();
            let right = this.padding_right.get();
            Rect { top, bottom, left, right }
        })
    }
}
