use nestix::{Layout, props};

use crate::{ClassList, Color, ViewProps};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[cfg(feature = "taffy")]
impl FlexDirection {
    pub fn to_taffy(&self) -> taffy::FlexDirection {
        match self {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        }
    }
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
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

#[cfg(feature = "taffy")]
impl FlexWrap {
    pub fn to_taffy(&self) -> taffy::FlexWrap {
        match self {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
        }
    }
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct FlexViewProps {
    #[props(default)]
    pub class: ClassList,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default)]
    pub children: Layout,

    #[props(default = FlexDirection::Column)]
    pub flex_direction: FlexDirection,
    #[props(default = AlignItems::Unset)]
    pub align_items: AlignItems,
    #[props(default = FlexWrap::NoWrap)]
    pub flex_wrap: FlexWrap,

    pub bg_color: Option<Color>,
}
