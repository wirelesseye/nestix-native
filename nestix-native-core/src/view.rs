use nestix::{Computed, computed, props};

use crate::{AlignItems, Length, Rect, WithAuto};

/// Layout properties shared by native visual controls.
#[props(
    debug,
    default,
    group(margin => [margin_left, margin_right, margin_top, margin_bottom]),
    group(margin_horizontal => [margin_left, margin_right]),
    group(margin_vertical => [margin_top, margin_bottom]),
)]
#[derive(Debug, Clone)]
pub struct ViewProps {
    /// Horizontal offset from the containing view.
    #[props(default = WithAuto::Auto)]
    pub left: WithAuto<Length>,
    /// Vertical offset from the containing view.
    #[props(default = WithAuto::Auto)]
    pub top: WithAuto<Length>,

    /// Requested width.
    #[props(default = WithAuto::Auto)]
    pub width: WithAuto<Length>,
    /// Requested height.
    #[props(default = WithAuto::Auto)]
    pub height: WithAuto<Length>,

    /// Margin on the left edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub margin_left: WithAuto<Length>,
    /// Margin on the right edge.
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub margin_right: WithAuto<Length>,
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub margin_top: WithAuto<Length>,
    #[props(default = WithAuto::Value(Length::logical(0)))]
    pub margin_bottom: WithAuto<Length>,

    /// Relative amount of free space the view may consume.
    #[props(default = 0.0)]
    pub flex_grow: f32,
    #[props(default = WithAuto::Auto)]
    pub flex_basis: WithAuto<Length>,
    #[props(default = 1.0)]
    pub flex_shrink: f32,
    #[props(default = AlignItems::Normal)]
    pub align_self: AlignItems,
}

impl ViewProps {
    /// Returns the four reactive margin values as a rectangle.
    pub fn margin(&self) -> Computed<Rect<WithAuto<Length>>> {
        computed!([this: self] || {
            let top = this.margin_top.get();
            let bottom = this.margin_bottom.get();
            let left = this.margin_left.get();
            let right = this.margin_right.get();
            Rect { top, bottom, left, right }
        })
    }
}
