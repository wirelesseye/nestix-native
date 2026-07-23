use dpi::LogicalUnit;
use nestix::{Computed, computed, props};

use crate::{AlignItems, Dimension, Rect};

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
    #[props(default = Dimension::Auto)]
    pub left: Dimension,
    /// Vertical offset from the containing view.
    #[props(default = Dimension::Auto)]
    pub top: Dimension,

    /// Requested width.
    #[props(default = Dimension::Auto)]
    pub width: Dimension,
    /// Requested height.
    #[props(default = Dimension::Auto)]
    pub height: Dimension,

    /// Margin on the left edge.
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_left: Dimension,
    /// Margin on the right edge.
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_right: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_top: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_bottom: Dimension,

    /// Relative amount of free space the view may consume.
    #[props(default = 0.0)]
    pub flex_grow: f32,
    #[props(default = Dimension::Auto)]
    pub flex_basis: Dimension,
    #[props(default = 1.0)]
    pub flex_shrink: f32,
    #[props(default = AlignItems::Normal)]
    pub align_self: AlignItems,
}

impl ViewProps {
    /// Returns the four reactive margin values as a rectangle.
    pub fn margin(&self) -> Computed<Rect<Dimension>> {
        computed!([this: self] || {
            let top = this.margin_top.get();
            let bottom = this.margin_bottom.get();
            let left = this.margin_left.get();
            let right = this.margin_right.get();
            Rect { top, bottom, left, right }
        })
    }
}
