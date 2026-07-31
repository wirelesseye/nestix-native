use nestix::{Computed, computed, props};

use crate::{AlignItems, Length, Rect, WithAuto};

/// Determines whether a view participates in normal layout flow.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Lay out the view normally, then apply any position offsets.
    #[default]
    Relative,
    /// Remove the view from normal layout flow and position it from its containing view.
    Absolute,
}

#[cfg(feature = "taffy")]
impl Position {
    pub fn to_taffy(&self) -> taffy::Position {
        match self {
            Self::Relative => taffy::Position::Relative,
            Self::Absolute => taffy::Position::Absolute,
        }
    }
}

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
    /// Whether the view participates in normal layout flow.
    #[props(default = Position::Relative)]
    pub position: Position,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_defaults_to_relative_and_accepts_absolute() {
        let default = ViewProps::default();
        assert_eq!(default.position.get(), Position::Relative);

        let absolute = nestix::build_props!(ViewProps(.position = Position::Absolute));
        assert_eq!(absolute.position.get(), Position::Absolute);
    }

    #[cfg(feature = "taffy")]
    #[test]
    fn position_maps_to_taffy() {
        assert_eq!(Position::Relative.to_taffy(), taffy::Position::Relative);
        assert_eq!(Position::Absolute.to_taffy(), taffy::Position::Absolute);
    }
}
