use nestix::{Computed, computed, props};

use crate::{Color, Length, Rect};

/// Border decoration shared by container-like native controls.
#[props(
    debug,
    default,
    group(width => [left_width, right_width, top_width, bottom_width]),
    group(horizontal_width => [left_width, right_width]),
    group(vertical_width => [top_width, bottom_width]),
)]
#[derive(Debug, Clone)]
pub struct BorderProps {
    /// Border width on the left edge.
    #[props(default = Length::logical(0))]
    pub left_width: Length,
    /// Border width on the right edge.
    #[props(default = Length::logical(0))]
    pub right_width: Length,
    /// Border width on the top edge.
    #[props(default = Length::logical(0))]
    pub top_width: Length,
    /// Border width on the bottom edge.
    #[props(default = Length::logical(0))]
    pub bottom_width: Length,
    /// Color shared by every border edge.
    pub color: Option<Color>,
    /// Radius applied to the border's outer corners.
    #[props(default = Length::logical(0))]
    pub radius: Length,
}

impl BorderProps {
    /// Returns the four reactive border widths as a rectangle.
    pub fn widths(&self) -> Computed<Rect<Length>> {
        computed!([this: self] || {
            Rect {
                top: this.top_width.get(),
                right: this.right_width.get(),
                bottom: this.bottom_width.get(),
                left: this.left_width.get(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_groups_set_the_expected_edges() {
        let all = nestix::build_props!(BorderProps(.width = 2));
        let horizontal = nestix::build_props!(BorderProps(.horizontal_width = 3));
        let vertical = nestix::build_props!(BorderProps(.vertical_width = 4));

        assert_eq!(
            all.widths().get(),
            Rect {
                top: Length::logical(2),
                right: Length::logical(2),
                bottom: Length::logical(2),
                left: Length::logical(2),
            }
        );
        assert_eq!(horizontal.left_width.get(), Length::logical(3));
        assert_eq!(horizontal.right_width.get(), Length::logical(3));
        assert_eq!(horizontal.top_width.get(), Length::logical(0));
        assert_eq!(horizontal.bottom_width.get(), Length::logical(0));
        assert_eq!(vertical.top_width.get(), Length::logical(4));
        assert_eq!(vertical.bottom_width.get(), Length::logical(4));
        assert_eq!(vertical.left_width.get(), Length::logical(0));
        assert_eq!(vertical.right_width.get(), Length::logical(0));
    }
}
