use nestix::{Layout, props};

use crate::{Color, ViewProps, ViewPropsExt, ViewPropsWrapper};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Unset,
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[cfg(feature = "taffy")]
impl AlignItems {
    pub fn to_taffy(&self) -> Option<taffy::AlignItems> {
        match self {
            AlignItems::Unset => None,
            AlignItems::FlexStart => Some(taffy::AlignItems::FlexStart),
            AlignItems::FlexEnd => Some(taffy::AlignItems::FlexEnd),
            AlignItems::Center => Some(taffy::AlignItems::Center),
            AlignItems::Start => Some(taffy::AlignItems::Start),
            AlignItems::End => Some(taffy::AlignItems::End),
            AlignItems::Baseline => Some(taffy::AlignItems::Baseline),
            AlignItems::Stretch => Some(taffy::AlignItems::Stretch),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    NoWrap,
    Wrap,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct FlexViewProps {
    #[props(extends(ViewPropsExt, ViewPropsWrapper))]
    view_props: ViewProps,

    #[props(default)]
    pub children: Layout,

    #[props(default = Direction::Column)]
    pub direction: Direction,
    #[props(default = AlignItems::Unset)]
    pub align_items: AlignItems,
    #[props(default = Wrap::NoWrap)]
    pub wrap: Wrap,

    pub background_color: Option<Color>,
}
