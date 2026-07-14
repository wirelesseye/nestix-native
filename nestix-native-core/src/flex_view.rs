use dpi::LogicalUnit;
use nestix::{Layout, props};

use crate::{ClassList, Color, ContainerProps, Dimension, ViewProps};

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
    Normal,
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
            AlignItems::Normal => None,
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
pub enum JustifyContent {
    Normal,
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[cfg(feature = "taffy")]
impl JustifyContent {
    pub fn to_taffy(&self) -> Option<taffy::JustifyContent> {
        match self {
            JustifyContent::Normal => None,
            JustifyContent::Start => Some(taffy::JustifyContent::Start),
            JustifyContent::End => Some(taffy::JustifyContent::End),
            JustifyContent::FlexStart => Some(taffy::JustifyContent::FlexStart),
            JustifyContent::FlexEnd => Some(taffy::JustifyContent::FlexEnd),
            JustifyContent::Center => Some(taffy::JustifyContent::Center),
            JustifyContent::Stretch => Some(taffy::JustifyContent::Stretch),
            JustifyContent::SpaceBetween => Some(taffy::JustifyContent::SpaceBetween),
            JustifyContent::SpaceEvenly => Some(taffy::JustifyContent::SpaceEvenly),
            JustifyContent::SpaceAround => Some(taffy::JustifyContent::SpaceAround),
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

    #[props(nested, default)]
    pub container: ContainerProps,

    #[props(default)]
    pub children: Layout,

    #[props(default = FlexDirection::Column)]
    pub flex_direction: FlexDirection,
    #[props(default = AlignItems::Normal)]
    pub align_items: AlignItems,
    #[props(default = JustifyContent::Normal)]
    pub justify_content: JustifyContent,
    #[props(default = FlexWrap::NoWrap)]
    pub flex_wrap: FlexWrap,

    #[props(default = Dimension::Length(LogicalUnit::new(0).into()))]
    pub gap: Dimension,

    pub bg_color: Option<Color>,
}
