use nestix::computed;

use crate::{Dimension, FlexViewProps, ViewProps};

use super::{
    ResolvedStyle, style_align_items, style_align_self, style_dimension, style_flex_basis,
    style_flex_direction, style_flex_grow, style_flex_shrink, style_flex_wrap, style_gap,
    style_justify_content,
};

/// Resolves the common view props once so the same effective values can be
/// rendered by the element and inherited by its descendants.
pub fn resolved_view_style(
    style: nestix::Computed<Option<ResolvedStyle>>,
    props: &ViewProps,
) -> nestix::Computed<Option<ResolvedStyle>> {
    computed!(
        [
            style,
            props.left,
            props.top,
            props.width,
            props.height,
            props.margin_left,
            props.margin_right,
            props.margin_top,
            props.margin_bottom,
            props.flex_grow,
            props.flex_basis,
            props.flex_shrink,
            props.align_self
        ] || {
            let mut resolved = style.get().unwrap_or_default();
            resolved.left = Some(style_dimension(
                Some(&resolved),
                left.get(),
                Dimension::Auto,
                |style| style.left,
            ));
            resolved.top = Some(style_dimension(
                Some(&resolved),
                top.get(),
                Dimension::Auto,
                |style| style.top,
            ));
            resolved.width = Some(style_dimension(
                Some(&resolved),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            ));
            resolved.height = Some(style_dimension(
                Some(&resolved),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            ));
            resolved.margin_left = Some(style_dimension(
                Some(&resolved),
                margin_left.get(),
                Dimension::from(0),
                |style| style.margin_left,
            ));
            resolved.margin_right = Some(style_dimension(
                Some(&resolved),
                margin_right.get(),
                Dimension::from(0),
                |style| style.margin_right,
            ));
            resolved.margin_top = Some(style_dimension(
                Some(&resolved),
                margin_top.get(),
                Dimension::from(0),
                |style| style.margin_top,
            ));
            resolved.margin_bottom = Some(style_dimension(
                Some(&resolved),
                margin_bottom.get(),
                Dimension::from(0),
                |style| style.margin_bottom,
            ));
            resolved.flex_grow = Some(style_flex_grow(Some(&resolved), flex_grow.get()));
            resolved.flex_basis = Some(style_flex_basis(Some(&resolved), flex_basis.get()));
            resolved.flex_shrink = Some(style_flex_shrink(Some(&resolved), flex_shrink.get()));
            resolved.align_self = Some(style_align_self(Some(&resolved), align_self.get()));
            Some(resolved)
        }
    )
}

/// Resolves view and flex-container props without duplicating their precedence
/// rules in each native backend.
pub fn resolved_flex_view_style(
    style: nestix::Computed<Option<ResolvedStyle>>,
    props: &FlexViewProps,
) -> nestix::Computed<Option<ResolvedStyle>> {
    let style = resolved_view_style(style, &props.view);
    computed!(
        [
            style,
            props.flex_direction,
            props.align_items,
            props.justify_content,
            props.flex_wrap,
            props.gap,
            props.padding_left,
            props.padding_right,
            props.padding_top,
            props.padding_bottom,
            props.bg_color
        ] || {
            let mut resolved = style.get().unwrap_or_default();
            resolved.flex_direction =
                Some(style_flex_direction(Some(&resolved), flex_direction.get()));
            resolved.align_items = Some(style_align_items(Some(&resolved), align_items.get()));
            resolved.justify_content = Some(style_justify_content(
                Some(&resolved),
                justify_content.get(),
            ));
            resolved.flex_wrap = Some(style_flex_wrap(Some(&resolved), flex_wrap.get()));
            resolved.gap = Some(style_gap(Some(&resolved), gap.get()));
            resolved.padding_left = Some(style_dimension(
                Some(&resolved),
                padding_left.get(),
                Dimension::from(0),
                |style| style.padding_left,
            ));
            resolved.padding_right = Some(style_dimension(
                Some(&resolved),
                padding_right.get(),
                Dimension::from(0),
                |style| style.padding_right,
            ));
            resolved.padding_top = Some(style_dimension(
                Some(&resolved),
                padding_top.get(),
                Dimension::from(0),
                |style| style.padding_top,
            ));
            resolved.padding_bottom = Some(style_dimension(
                Some(&resolved),
                padding_bottom.get(),
                Dimension::from(0),
                |style| style.padding_bottom,
            ));
            resolved.bg_color = bg_color.get().or(resolved.bg_color);
            Some(resolved)
        }
    )
}
