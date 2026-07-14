use dpi::LogicalUnit;
use nestix::{Computed, PropValue, computed, props};

use crate::{Dimension, Rect};

#[props(
    debug,
    default,
    group(padding => [padding_left, padding_right, padding_top, padding_bottom]),
    group(padding_horizontal => [padding_left, padding_right]),
    group(padding_vertical => [padding_top, padding_bottom]),
)]
#[derive(Debug, Clone)]
pub struct ContainerProps {
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub padding_left: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub padding_right: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub padding_top: Dimension,
    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub padding_bottom: Dimension,
}

impl ContainerProps {
    pub(crate) fn auto_padding() -> Self {
        Self::builder()
            .padding(PropValue::from_plain(Dimension::Auto))
            .build()
    }

    pub fn padding(&self) -> Computed<Rect<Dimension>> {
        computed!([this: self] || {
            let top = this.padding_top.get();
            let bottom = this.padding_bottom.get();
            let left = this.padding_left.get();
            let right = this.padding_right.get();
            Rect { top, bottom, left, right }
        })
    }
}
