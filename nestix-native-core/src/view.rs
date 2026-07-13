use dpi::LogicalUnit;
use nestix::{Computed, computed, props};

use crate::{AlignItems, Dimension, Rect};

#[props(
    debug,
    default,
    group(margin => [margin_left, margin_right, margin_top, margin_bottom]),
    group(margin_horizontal => [margin_left, margin_right]),
    group(margin_vertical => [margin_top, margin_bottom]),
)]
#[derive(Debug, Clone)]
pub struct ViewProps {
    #[props(default = Dimension::Auto)]
    pub left: Dimension,
    #[props(default = Dimension::Auto)]
    pub top: Dimension,

    #[props(default = Dimension::Auto)]
    pub width: Dimension,
    #[props(default = Dimension::Auto)]
    pub height: Dimension,

    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_left: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_right: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_top: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub margin_bottom: Dimension,

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
