use nestix::{Layout, props};

use crate::{Color, ExtendsViewProps, ViewProps};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Unset,
    FlexStart,
    FlexEnd,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wrap {
    NoWrap,
    Wrap,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct FlexViewProps {
    #[props(extends(ExtendsViewProps))]
    view_props: ViewProps,

    #[props(default)]
    pub children: Layout,

    #[props(default = Direction::Column)]
    pub direction: Direction,
    #[props(default = Alignment::Unset)]
    pub alignment: Alignment,
    #[props(default = Wrap::NoWrap)]
    pub wrap: Wrap,
    
    pub background_color: Option<Color>,
}
