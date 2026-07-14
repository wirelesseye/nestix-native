/// Identifies a built-in stylesheet property independently of its typed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StylePropertyName {
    Appearance,
    BgColor,
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    TextColor,
    Left,
    Top,
    Width,
    Height,
    Margin,
    MarginHorizontal,
    MarginVertical,
    MarginLeft,
    MarginRight,
    MarginTop,
    MarginBottom,
    Padding,
    PaddingHorizontal,
    PaddingVertical,
    PaddingLeft,
    PaddingRight,
    PaddingTop,
    PaddingBottom,
    FlexGrow,
    FlexBasis,
    FlexShrink,
    AlignSelf,
    FlexDirection,
    AlignItems,
    JustifyContent,
    FlexWrap,
    Gap,
}

impl StylePropertyName {
    pub fn name(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::BgColor => "bg_color",
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::FontWeight => "font_weight",
            Self::FontStyle => "font_style",
            Self::TextColor => "text_color",
            Self::Left => "left",
            Self::Top => "top",
            Self::Width => "width",
            Self::Height => "height",
            Self::Margin => "margin",
            Self::MarginHorizontal => "margin_horizontal",
            Self::MarginVertical => "margin_vertical",
            Self::MarginLeft => "margin_left",
            Self::MarginRight => "margin_right",
            Self::MarginTop => "margin_top",
            Self::MarginBottom => "margin_bottom",
            Self::Padding => "padding",
            Self::PaddingHorizontal => "padding_horizontal",
            Self::PaddingVertical => "padding_vertical",
            Self::PaddingLeft => "padding_left",
            Self::PaddingRight => "padding_right",
            Self::PaddingTop => "padding_top",
            Self::PaddingBottom => "padding_bottom",
            Self::FlexGrow => "flex_grow",
            Self::FlexBasis => "flex_basis",
            Self::FlexShrink => "flex_shrink",
            Self::AlignSelf => "align_self",
            Self::FlexDirection => "flex_direction",
            Self::AlignItems => "align_items",
            Self::JustifyContent => "justify_content",
            Self::FlexWrap => "flex_wrap",
            Self::Gap => "gap",
        }
    }

    pub(super) fn affected_names(self) -> &'static [&'static str] {
        match self {
            Self::Appearance => &["appearance"],
            Self::BgColor => &["bg_color"],
            Self::FontFamily => &["font_family"],
            Self::FontSize => &["font_size"],
            Self::FontWeight => &["font_weight"],
            Self::FontStyle => &["font_style"],
            Self::TextColor => &["text_color"],
            Self::Left => &["left"],
            Self::Top => &["top"],
            Self::Width => &["width"],
            Self::Height => &["height"],
            Self::Margin => &["margin_left", "margin_right", "margin_top", "margin_bottom"],
            Self::MarginHorizontal => &["margin_left", "margin_right"],
            Self::MarginVertical => &["margin_top", "margin_bottom"],
            Self::MarginLeft => &["margin_left"],
            Self::MarginRight => &["margin_right"],
            Self::MarginTop => &["margin_top"],
            Self::MarginBottom => &["margin_bottom"],
            Self::Padding => &[
                "padding_left",
                "padding_right",
                "padding_top",
                "padding_bottom",
            ],
            Self::PaddingHorizontal => &["padding_left", "padding_right"],
            Self::PaddingVertical => &["padding_top", "padding_bottom"],
            Self::PaddingLeft => &["padding_left"],
            Self::PaddingRight => &["padding_right"],
            Self::PaddingTop => &["padding_top"],
            Self::PaddingBottom => &["padding_bottom"],
            Self::FlexGrow => &["flex_grow"],
            Self::FlexBasis => &["flex_basis"],
            Self::FlexShrink => &["flex_shrink"],
            Self::AlignSelf => &["align_self"],
            Self::FlexDirection => &["flex_direction"],
            Self::AlignItems => &["align_items"],
            Self::JustifyContent => &["justify_content"],
            Self::FlexWrap => &["flex_wrap"],
            Self::Gap => &["gap"],
        }
    }

    pub(super) fn naturally_inherits(self) -> bool {
        matches!(
            self,
            Self::FontFamily
                | Self::FontSize
                | Self::FontWeight
                | Self::FontStyle
                | Self::TextColor
        )
    }
}

/// A typed stylesheet value or one of the stylesheet-only global values.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue<T> {
    Value(T),
    Inherit,
    Initial,
    Unset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlobalStyleValue {
    Inherit,
    Initial,
    Unset,
}

impl<T> StyleValue<T> {
    pub(super) fn global(&self) -> Option<GlobalStyleValue> {
        match self {
            Self::Value(_) => None,
            Self::Inherit => Some(GlobalStyleValue::Inherit),
            Self::Initial => Some(GlobalStyleValue::Initial),
            Self::Unset => Some(GlobalStyleValue::Unset),
        }
    }
}
