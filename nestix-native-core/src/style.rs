mod effective;
mod property;

pub use effective::{resolved_flex_view_style, resolved_view_style};
pub use property::StyleValue;
use property::{GlobalStyleValue, StylePropertyName};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use nestix::{
    ContextProvider, Element, Layout, PropValue, State, StateSetter, component, computed,
    create_state, layout, props,
};

use crate::{
    AlignItems, AnimationSpec, Appearance, Color, FlexDirection, FlexWrap, FontStyle, FontWeight,
    JustifyContent, Length, Position, Rect, ResolvedFontProps, WithAuto,
};

/// Cross-platform fallback used when a backend cannot determine its system UI font size.
pub const DEFAULT_ROOT_FONT_SIZE: f64 = 16.0;

/// A geometry property that can be transitioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    Left,
    Top,
    Width,
    Height,
    MarginLeft,
    MarginRight,
    MarginTop,
    MarginBottom,
    PaddingLeft,
    PaddingRight,
    PaddingTop,
    PaddingBottom,
    Gap,
}

impl TransitionProperty {
    pub const GEOMETRY: [Self; 13] = [
        Self::Left,
        Self::Top,
        Self::Width,
        Self::Height,
        Self::MarginLeft,
        Self::MarginRight,
        Self::MarginTop,
        Self::MarginBottom,
        Self::PaddingLeft,
        Self::PaddingRight,
        Self::PaddingTop,
        Self::PaddingBottom,
        Self::Gap,
    ];

    pub(crate) fn length_with_auto(self, style: &ResolvedStyle) -> Option<WithAuto<Length>> {
        match self {
            Self::Left => style.left,
            Self::Top => style.top,
            Self::Width => style.width,
            Self::Height => style.height,
            Self::MarginLeft => style.margin_left,
            Self::MarginRight => style.margin_right,
            Self::MarginTop => style.margin_top,
            Self::MarginBottom => style.margin_bottom,
            Self::PaddingLeft => style.padding_left,
            Self::PaddingRight => style.padding_right,
            Self::PaddingTop => style.padding_top,
            Self::PaddingBottom => style.padding_bottom,
            Self::Gap => style.gap,
        }
    }

    pub(crate) fn set_length_with_auto(self, style: &mut ResolvedStyle, value: WithAuto<Length>) {
        *match self {
            Self::Left => &mut style.left,
            Self::Top => &mut style.top,
            Self::Width => &mut style.width,
            Self::Height => &mut style.height,
            Self::MarginLeft => &mut style.margin_left,
            Self::MarginRight => &mut style.margin_right,
            Self::MarginTop => &mut style.margin_top,
            Self::MarginBottom => &mut style.margin_bottom,
            Self::PaddingLeft => &mut style.padding_left,
            Self::PaddingRight => &mut style.padding_right,
            Self::PaddingTop => &mut style.padding_top,
            Self::PaddingBottom => &mut style.padding_bottom,
            Self::Gap => &mut style.gap,
        } = Some(value);
    }
}

/// One property entry in a stylesheet transition declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleTransition {
    pub property: TransitionProperty,
    pub animation: AnimationSpec,
}

/// A whitespace-separated set of style class names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassList(HashSet<String>);

#[cfg(feature = "inspector")]
impl nestix::InspectableValue for ClassList {
    fn inspect_value(&self) -> nestix::InspectValue {
        let mut classes = self.0.iter().cloned().collect::<Vec<_>>();
        classes.sort();
        nestix::InspectValue::Display(classes.join(" "))
    }
}

impl ClassList {
    /// Returns whether this list contains `class`.
    pub fn contains(&self, class: &str) -> bool {
        self.0.contains(class)
    }

    /// Returns whether every class in `other` is present in this list.
    pub fn is_superset(&self, other: &ClassList) -> bool {
        self.0.is_superset(&other.0)
    }

    /// Returns a copy with the renderer's default classes added.
    pub fn with_defaults(&self, defaults: &[&str]) -> Self {
        let mut classes = self.0.clone();
        classes.extend(defaults.iter().map(|class| (*class).to_string()));
        Self(classes)
    }
}

impl From<&str> for ClassList {
    fn from(value: &str) -> Self {
        Self(value.split_whitespace().map(str::to_owned).collect())
    }
}

impl From<String> for ClassList {
    fn from(value: String) -> Self {
        Self(value.split_whitespace().map(str::to_owned).collect())
    }
}

impl From<HashSet<String>> for ClassList {
    fn from(value: HashSet<String>) -> Self {
        Self(value)
    }
}

/// A selector used to match a [`StyleRule`] against an element context.
#[derive(Debug, Clone)]
pub enum StyleSelector {
    /// Matches an element containing the given class.
    Class(String),
    /// Matches when the nested selector does not match.
    Not(Box<StyleSelector>),
    /// Matches the first child of its parent.
    FirstChild,
    /// Matches the last child of its parent.
    LastChild,
    /// Matches children whose one-based index satisfies `a * n + b`.
    NthChild {
        /// Step size of the arithmetic sequence.
        a: isize,
        /// Offset of the arithmetic sequence.
        b: isize,
    },
    /// Matches when every nested selector matches the same element.
    All(Vec<StyleSelector>),
    /// Matches a child and its direct parent.
    Child {
        /// Selector applied to the direct parent.
        parent: Box<StyleSelector>,
        /// Selector applied to the child.
        child: Box<StyleSelector>,
    },
    /// Matches an element with any matching ancestor.
    Descendant {
        /// Selector applied to an ancestor.
        ancestor: Box<StyleSelector>,
        /// Selector applied to the descendant element.
        descendant: Box<StyleSelector>,
    },
    /// Matches an element immediately preceded by a matching sibling.
    AdjacentSibling {
        /// Selector applied to the immediately preceding sibling.
        previous: Box<StyleSelector>,
        /// Selector applied to the current sibling.
        sibling: Box<StyleSelector>,
    },
    /// Matches an element preceded by any matching sibling.
    SubsequentSibling {
        /// Selector applied to an earlier sibling.
        previous: Box<StyleSelector>,
        /// Selector applied to the current sibling.
        sibling: Box<StyleSelector>,
    },
    /// Matches when any nested selector matches.
    List(Vec<StyleSelector>),
}

impl StyleSelector {
    /// Returns whether this selector matches `context`.
    pub fn matches(&self, context: &MatchContext) -> bool {
        self.matched_specificity(context).is_some()
    }

    /// Returns the selector specificity when it matches `context`.
    pub fn matched_specificity(&self, context: &MatchContext) -> Option<usize> {
        match self {
            StyleSelector::Class(class) if context.class_list.contains(class) => Some(1),
            StyleSelector::Class(_) => None,
            StyleSelector::Not(selector) => {
                if selector.matches(context) {
                    None
                } else {
                    Some(selector.specificity())
                }
            }
            StyleSelector::FirstChild
                if context
                    .child_position()
                    .is_some_and(|(index, _)| index == 1) =>
            {
                Some(1)
            }
            StyleSelector::FirstChild => None,
            StyleSelector::LastChild
                if context
                    .child_position()
                    .is_some_and(|(index, count)| index == count) =>
            {
                Some(1)
            }
            StyleSelector::LastChild => None,
            StyleSelector::NthChild { a, b }
                if context
                    .child_position()
                    .is_some_and(|(index, _)| nth_child_matches(index, *a, *b)) =>
            {
                Some(1)
            }
            StyleSelector::NthChild { .. } => None,
            StyleSelector::All(selectors) => {
                selectors.iter().try_fold(0, |specificity, selector| {
                    selector
                        .matched_specificity(context)
                        .map(|selector_specificity| specificity + selector_specificity)
                })
            }
            StyleSelector::Child { parent, child } => {
                let child_specificity = child.matched_specificity(context)?;
                let parent_context = context.parent()?;
                let parent_specificity = parent.matched_specificity(&parent_context)?;
                Some(parent_specificity + child_specificity)
            }
            StyleSelector::Descendant {
                ancestor,
                descendant,
            } => {
                let descendant_specificity = descendant.matched_specificity(context)?;

                context
                    .ancestors
                    .iter()
                    .enumerate()
                    .filter_map(|(index, ancestor_class_list)| {
                        let ancestor_context = context.ancestor_at(index, ancestor_class_list);
                        ancestor.matched_specificity(&ancestor_context).map(
                            |ancestor_specificity| ancestor_specificity + descendant_specificity,
                        )
                    })
                    .max()
            }
            StyleSelector::AdjacentSibling { previous, sibling } => {
                let sibling_specificity = sibling.matched_specificity(context)?;
                let previous_context = context.previous_sibling()?;
                let previous_specificity = previous.matched_specificity(&previous_context)?;
                Some(previous_specificity + sibling_specificity)
            }
            StyleSelector::SubsequentSibling { previous, sibling } => {
                let sibling_specificity = sibling.matched_specificity(context)?;

                context
                    .previous_siblings
                    .iter()
                    .enumerate()
                    .filter_map(|(index, previous_class_list)| {
                        let previous_context =
                            context.previous_sibling_at(index, previous_class_list);
                        previous
                            .matched_specificity(&previous_context)
                            .map(|previous_specificity| previous_specificity + sibling_specificity)
                    })
                    .max()
            }
            StyleSelector::List(selectors) => selectors
                .iter()
                .filter_map(|selector| selector.matched_specificity(context))
                .max(),
        }
    }

    fn specificity(&self) -> usize {
        match self {
            StyleSelector::Class(_) => 1,
            StyleSelector::Not(selector) => selector.specificity(),
            StyleSelector::FirstChild
            | StyleSelector::LastChild
            | StyleSelector::NthChild { .. } => 1,
            StyleSelector::All(selectors) => selectors.iter().map(Self::specificity).sum(),
            StyleSelector::Child { parent, child } => parent.specificity() + child.specificity(),
            StyleSelector::Descendant {
                ancestor,
                descendant,
            } => ancestor.specificity() + descendant.specificity(),
            StyleSelector::AdjacentSibling { previous, sibling } => {
                previous.specificity() + sibling.specificity()
            }
            StyleSelector::SubsequentSibling { previous, sibling } => {
                previous.specificity() + sibling.specificity()
            }
            StyleSelector::List(selectors) => {
                selectors.iter().map(Self::specificity).max().unwrap_or(0)
            }
        }
    }

    fn uses_structural_position(&self) -> bool {
        match self {
            Self::FirstChild | Self::LastChild | Self::NthChild { .. } => true,
            Self::Not(selector) => selector.uses_structural_position(),
            Self::All(selectors) | Self::List(selectors) => {
                selectors.iter().any(Self::uses_structural_position)
            }
            Self::Child { parent, child } => {
                parent.uses_structural_position() || child.uses_structural_position()
            }
            Self::Descendant {
                ancestor,
                descendant,
            } => ancestor.uses_structural_position() || descendant.uses_structural_position(),
            Self::AdjacentSibling { previous, sibling }
            | Self::SubsequentSibling { previous, sibling } => {
                previous.uses_structural_position() || sibling.uses_structural_position()
            }
            Self::Class(_) => false,
        }
    }
}

fn nth_child_matches(index: usize, a: isize, b: isize) -> bool {
    let index = index as i128;
    let a = a as i128;
    let b = b as i128;
    if a == 0 {
        return index == b;
    }

    let difference = index - b;
    difference % a == 0 && difference / a >= 0
}

/// A built-in style property and its declared value.
#[derive(Debug, Clone)]
pub enum StyleProperty {
    /// Whether a component uses its backend-native theme.
    ///
    /// **Available value**: `native`, `none`, or `auto`.
    Appearance(StyleValue<Appearance>),
    /// Background color applied to the element.
    ///
    /// **Available value**: a named color (`white`, `black`, `transparent`, `red`,
    /// `green`, `blue`), or a 6/8 digit hex color (`#RRGGBB` or `#RRGGBBAA`).
    BgColor(StyleValue<Color>),
    /// Color shared by all border edges.
    BorderColor(StyleValue<Color>),
    /// Radius applied to the border's outer corners.
    BorderRadius(StyleValue<Length>),
    /// Border width applied to all four edges.
    BorderWidth(StyleValue<Length>),
    /// Border width applied to the left and right edges.
    BorderHorizontalWidth(StyleValue<Length>),
    /// Border width applied to the top and bottom edges.
    BorderVerticalWidth(StyleValue<Length>),
    /// Border width applied to the left edge.
    BorderLeftWidth(StyleValue<Length>),
    /// Border width applied to the right edge.
    BorderRightWidth(StyleValue<Length>),
    /// Border width applied to the top edge.
    BorderTopWidth(StyleValue<Length>),
    /// Border width applied to the bottom edge.
    BorderBottomWidth(StyleValue<Length>),
    /// Font family name. This property is inherited.
    ///
    /// **Available value**: a single-word family name (`Arial`), a double-quoted
    /// family name (`"Comic Sans"`).
    FontFamily(StyleValue<String>),
    /// Font size in logical pixels. This property is inherited.
    ///
    /// **Available value**: pixels such as `14 px`, or a font-relative value such as `1.2 em`.
    FontSize(StyleValue<Length>),
    /// Font weight. This property is inherited.
    ///
    /// **Available value**: `thin`, `extra-light`, `light`, `normal`, `medium`,
    /// `semi-bold`, `bold`, `extra-bold`, `black`, or a numeric weight from `1`
    /// through `1000`.
    FontWeight(StyleValue<FontWeight>),
    /// Font style. This property is inherited.
    ///
    /// **Available value**: `normal`, `italic`.
    FontStyle(StyleValue<FontStyle>),
    /// Foreground text color. This property is inherited.
    ///
    /// **Available value**: a named color (`white`, `black`, `transparent`, `red`,
    /// `green`, `blue`), or a 6/8 digit hex color (`#RRGGBB` or `#RRGGBBAA`).
    TextColor(StyleValue<Color>),
    /// Whether the element participates in normal layout flow.
    ///
    /// **Available value**: `relative` or `absolute`.
    Position(StyleValue<Position>),
    /// Horizontal position offset from the left edge of the containing block.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Left(StyleValue<WithAuto<Length>>),
    /// Vertical position offset from the top edge of the containing block.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Top(StyleValue<WithAuto<Length>>),
    /// Preferred layout width.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Width(StyleValue<WithAuto<Length>>),
    /// Preferred layout height.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Height(StyleValue<WithAuto<Length>>),
    /// Margin applied to all four edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Margin(StyleValue<WithAuto<Length>>),
    /// Margin applied to the left and right edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginHorizontal(StyleValue<WithAuto<Length>>),
    /// Margin applied to the top and bottom edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginVertical(StyleValue<WithAuto<Length>>),
    /// Margin applied to the left edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginLeft(StyleValue<WithAuto<Length>>),
    /// Margin applied to the right edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginRight(StyleValue<WithAuto<Length>>),
    /// Margin applied to the top edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginTop(StyleValue<WithAuto<Length>>),
    /// Margin applied to the bottom edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    MarginBottom(StyleValue<WithAuto<Length>>),
    /// Padding applied to all four edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Padding(StyleValue<WithAuto<Length>>),
    /// Padding applied to the left and right edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingHorizontal(StyleValue<WithAuto<Length>>),
    /// Padding applied to the top and bottom edges.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingVertical(StyleValue<WithAuto<Length>>),
    /// Padding applied to the left edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingLeft(StyleValue<WithAuto<Length>>),
    /// Padding applied to the right edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingRight(StyleValue<WithAuto<Length>>),
    /// Padding applied to the top edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingTop(StyleValue<WithAuto<Length>>),
    /// Padding applied to the bottom edge.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    PaddingBottom(StyleValue<WithAuto<Length>>),
    /// Flex grow factor used when distributing free space.
    ///
    /// **Available value**: a number.
    FlexGrow(StyleValue<f32>),
    /// Initial main size of the flex item.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    FlexBasis(StyleValue<WithAuto<Length>>),
    /// Flex shrink factor used when distributing negative free space.
    ///
    /// **Available value**: a number.
    FlexShrink(StyleValue<f32>),
    /// Cross-axis alignment override for this element within its flex parent.
    ///
    /// **Available value**: `normal`, `start`, `end`, `flex-start`, `flex-end`,
    /// `center`, `baseline`, or `stretch`.
    AlignSelf(StyleValue<AlignItems>),
    /// Main-axis direction for this element's flex children.
    ///
    /// **Available value**: `row`, `row-reverse`, `column`, or `column-reverse`.
    FlexDirection(StyleValue<FlexDirection>),
    /// Cross-axis alignment for this element's flex children.
    ///
    /// **Available value**: `normal`, `start`, `end`, `flex-start`, `flex-end`,
    /// `center`, `baseline`, or `stretch`.
    AlignItems(StyleValue<AlignItems>),
    /// Main-axis distribution for this element's flex children.
    ///
    /// **Available value**: `normal`, `start`, `end`, `flex-start`, `flex-end`,
    /// `center`, `stretch`, `space-between`, `space-evenly`, or `space-around`.
    JustifyContent(StyleValue<JustifyContent>),
    /// Wrapping behavior for this element's flex children.
    ///
    /// **Available value**: `nowrap`, `no-wrap`, or `wrap`.
    FlexWrap(StyleValue<FlexWrap>),
    /// Spacing between this element's flex children.
    ///
    /// **Available value**: `auto`, pixels such as `30 px`, or font-relative values such as `2 em`.
    Gap(StyleValue<WithAuto<Length>>),
    /// Geometry transitions applied when resolved targets change.
    ///
    /// **Available value**: a comma-separated list of `<property> <duration> [easing]`
    /// entries, such as `width 200ms ease_out`, or the `layout` property shorthand.
    Transition(StyleValue<Vec<StyleTransition>>),
}

impl StyleProperty {
    pub(crate) fn property_name(&self) -> StylePropertyName {
        match self {
            Self::Appearance(_) => StylePropertyName::Appearance,
            Self::BgColor(_) => StylePropertyName::BgColor,
            Self::BorderColor(_) => StylePropertyName::BorderColor,
            Self::BorderRadius(_) => StylePropertyName::BorderRadius,
            Self::BorderWidth(_) => StylePropertyName::BorderWidth,
            Self::BorderHorizontalWidth(_) => StylePropertyName::BorderHorizontalWidth,
            Self::BorderVerticalWidth(_) => StylePropertyName::BorderVerticalWidth,
            Self::BorderLeftWidth(_) => StylePropertyName::BorderLeftWidth,
            Self::BorderRightWidth(_) => StylePropertyName::BorderRightWidth,
            Self::BorderTopWidth(_) => StylePropertyName::BorderTopWidth,
            Self::BorderBottomWidth(_) => StylePropertyName::BorderBottomWidth,
            Self::FontFamily(_) => StylePropertyName::FontFamily,
            Self::FontSize(_) => StylePropertyName::FontSize,
            Self::FontWeight(_) => StylePropertyName::FontWeight,
            Self::FontStyle(_) => StylePropertyName::FontStyle,
            Self::TextColor(_) => StylePropertyName::TextColor,
            Self::Position(_) => StylePropertyName::Position,
            Self::Left(_) => StylePropertyName::Left,
            Self::Top(_) => StylePropertyName::Top,
            Self::Width(_) => StylePropertyName::Width,
            Self::Height(_) => StylePropertyName::Height,
            Self::Margin(_) => StylePropertyName::Margin,
            Self::MarginHorizontal(_) => StylePropertyName::MarginHorizontal,
            Self::MarginVertical(_) => StylePropertyName::MarginVertical,
            Self::MarginLeft(_) => StylePropertyName::MarginLeft,
            Self::MarginRight(_) => StylePropertyName::MarginRight,
            Self::MarginTop(_) => StylePropertyName::MarginTop,
            Self::MarginBottom(_) => StylePropertyName::MarginBottom,
            Self::Padding(_) => StylePropertyName::Padding,
            Self::PaddingHorizontal(_) => StylePropertyName::PaddingHorizontal,
            Self::PaddingVertical(_) => StylePropertyName::PaddingVertical,
            Self::PaddingLeft(_) => StylePropertyName::PaddingLeft,
            Self::PaddingRight(_) => StylePropertyName::PaddingRight,
            Self::PaddingTop(_) => StylePropertyName::PaddingTop,
            Self::PaddingBottom(_) => StylePropertyName::PaddingBottom,
            Self::FlexGrow(_) => StylePropertyName::FlexGrow,
            Self::FlexBasis(_) => StylePropertyName::FlexBasis,
            Self::FlexShrink(_) => StylePropertyName::FlexShrink,
            Self::AlignSelf(_) => StylePropertyName::AlignSelf,
            Self::FlexDirection(_) => StylePropertyName::FlexDirection,
            Self::AlignItems(_) => StylePropertyName::AlignItems,
            Self::JustifyContent(_) => StylePropertyName::JustifyContent,
            Self::FlexWrap(_) => StylePropertyName::FlexWrap,
            Self::Gap(_) => StylePropertyName::Gap,
            Self::Transition(_) => StylePropertyName::Transition,
        }
    }

    /// Returns the source-language name of this property.
    pub fn name(&self) -> &'static str {
        self.property_name().name()
    }

    fn global(&self) -> Option<GlobalStyleValue> {
        match self {
            Self::Appearance(value) => value.global(),
            Self::BgColor(value) => value.global(),
            Self::BorderColor(value) => value.global(),
            Self::BorderRadius(value) => value.global(),
            Self::BorderWidth(value) => value.global(),
            Self::BorderHorizontalWidth(value) => value.global(),
            Self::BorderVerticalWidth(value) => value.global(),
            Self::BorderLeftWidth(value) => value.global(),
            Self::BorderRightWidth(value) => value.global(),
            Self::BorderTopWidth(value) => value.global(),
            Self::BorderBottomWidth(value) => value.global(),
            Self::FontFamily(value) => value.global(),
            Self::FontSize(value) => value.global(),
            Self::FontWeight(value) => value.global(),
            Self::FontStyle(value) => value.global(),
            Self::TextColor(value) => value.global(),
            Self::Position(value) => value.global(),
            Self::Left(value) => value.global(),
            Self::Top(value) => value.global(),
            Self::Width(value) => value.global(),
            Self::Height(value) => value.global(),
            Self::Margin(value) => value.global(),
            Self::MarginHorizontal(value) => value.global(),
            Self::MarginVertical(value) => value.global(),
            Self::MarginLeft(value) => value.global(),
            Self::MarginRight(value) => value.global(),
            Self::MarginTop(value) => value.global(),
            Self::MarginBottom(value) => value.global(),
            Self::Padding(value) => value.global(),
            Self::PaddingHorizontal(value) => value.global(),
            Self::PaddingVertical(value) => value.global(),
            Self::PaddingLeft(value) => value.global(),
            Self::PaddingRight(value) => value.global(),
            Self::PaddingTop(value) => value.global(),
            Self::PaddingBottom(value) => value.global(),
            Self::FlexGrow(value) => value.global(),
            Self::FlexBasis(value) => value.global(),
            Self::FlexShrink(value) => value.global(),
            Self::AlignSelf(value) => value.global(),
            Self::FlexDirection(value) => value.global(),
            Self::AlignItems(value) => value.global(),
            Self::JustifyContent(value) => value.global(),
            Self::FlexWrap(value) => value.global(),
            Self::Gap(value) => value.global(),
            Self::Transition(value) => value.global(),
        }
    }
}

/// A declaration contained in a [`StyleRule`].
#[derive(Debug, Clone)]
pub enum StyleDeclaration {
    /// A declaration for a built-in property.
    Property(StyleProperty),
    /// A custom property retained as a string.
    Custom {
        /// Custom property name.
        name: String,
        /// Custom property value.
        value: String,
    },
}

impl StyleDeclaration {
    fn affected_names(&self) -> Vec<&str> {
        match self {
            StyleDeclaration::Property(property) => {
                property.property_name().affected_names().to_vec()
            }
            StyleDeclaration::Custom { name, .. } => vec![name.as_str()],
        }
    }
}

/// A selector and the declarations applied when it matches.
#[derive(Debug, Clone)]
pub struct StyleRule {
    /// Selector controlling where the declarations apply.
    pub selector: StyleSelector,
    /// Declarations in source order.
    pub declarations: Vec<StyleDeclaration>,
}

/// Effective values produced by resolving matching declarations.
#[derive(Debug, Clone, Default, PartialEq, nestix::InspectableValue)]
pub struct ResolvedStyle {
    /// Resolved native appearance mode.
    pub appearance: Option<Appearance>,
    /// Resolved background color.
    pub bg_color: Option<Color>,
    /// Resolved border color.
    pub border_color: Option<Color>,
    /// Resolved border corner radius.
    pub border_radius: Option<Length>,
    /// Resolved left border width.
    pub border_left_width: Option<Length>,
    /// Resolved right border width.
    pub border_right_width: Option<Length>,
    /// Resolved top border width.
    pub border_top_width: Option<Length>,
    /// Resolved bottom border width.
    pub border_bottom_width: Option<Length>,
    /// Resolved font family.
    pub font_family: Option<String>,
    /// Resolved font size.
    pub font_size: Option<f64>,
    /// Resolved font weight.
    pub font_weight: Option<FontWeight>,
    /// Resolved font style.
    pub font_style: Option<FontStyle>,
    /// Resolved foreground text color.
    pub text_color: Option<Color>,
    /// Resolved layout positioning mode.
    pub position: Option<Position>,
    /// Resolved left offset.
    pub left: Option<WithAuto<Length>>,
    /// Resolved top offset.
    pub top: Option<WithAuto<Length>>,
    /// Resolved width.
    pub width: Option<WithAuto<Length>>,
    /// Resolved height.
    pub height: Option<WithAuto<Length>>,
    /// Resolved left margin.
    pub margin_left: Option<WithAuto<Length>>,
    /// Resolved right margin.
    pub margin_right: Option<WithAuto<Length>>,
    /// Resolved top margin.
    pub margin_top: Option<WithAuto<Length>>,
    /// Resolved bottom margin.
    pub margin_bottom: Option<WithAuto<Length>>,
    /// Resolved left padding.
    pub padding_left: Option<WithAuto<Length>>,
    /// Resolved right padding.
    pub padding_right: Option<WithAuto<Length>>,
    /// Resolved top padding.
    pub padding_top: Option<WithAuto<Length>>,
    /// Resolved bottom padding.
    pub padding_bottom: Option<WithAuto<Length>>,
    /// Resolved flex grow factor.
    pub flex_grow: Option<f32>,
    /// Resolved flex basis.
    pub flex_basis: Option<WithAuto<Length>>,
    /// Resolved flex shrink factor.
    pub flex_shrink: Option<f32>,
    /// Resolved cross-axis alignment override.
    pub align_self: Option<AlignItems>,
    /// Resolved main-axis direction.
    pub flex_direction: Option<FlexDirection>,
    /// Resolved child cross-axis alignment.
    pub align_items: Option<AlignItems>,
    /// Resolved child main-axis distribution.
    pub justify_content: Option<JustifyContent>,
    /// Resolved flex wrapping mode.
    pub flex_wrap: Option<FlexWrap>,
    /// Resolved spacing between flex children.
    pub gap: Option<WithAuto<Length>>,
    /// Transitions applied to changing geometry values.
    pub transitions: Vec<StyleTransition>,
    custom: HashMap<String, String>,
}

impl ResolvedStyle {
    fn resolve_em_lengths(&mut self) {
        let font_size = self.font_size.unwrap_or(DEFAULT_ROOT_FONT_SIZE);
        for value in [
            &mut self.left,
            &mut self.top,
            &mut self.width,
            &mut self.height,
            &mut self.margin_left,
            &mut self.margin_right,
            &mut self.margin_top,
            &mut self.margin_bottom,
            &mut self.padding_left,
            &mut self.padding_right,
            &mut self.padding_top,
            &mut self.padding_bottom,
            &mut self.flex_basis,
            &mut self.gap,
        ] {
            if let Some(WithAuto::Value(length)) = value {
                *length = length.resolve(font_size);
            }
        }
        for length in [
            &mut self.border_radius,
            &mut self.border_left_width,
            &mut self.border_right_width,
            &mut self.border_top_width,
            &mut self.border_bottom_width,
        ] {
            if let Some(length) = length {
                *length = length.resolve(font_size);
            }
        }
    }

    pub fn transition_for(&self, property: TransitionProperty) -> Option<AnimationSpec> {
        self.transitions
            .iter()
            .rev()
            .find(|transition| transition.property == property)
            .map(|transition| transition.animation)
    }
    /// Extracts the resolved inherited font properties.
    pub fn font(&self) -> ResolvedFontProps {
        ResolvedFontProps {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
            text_color: self.text_color,
        }
    }

    /// Returns a custom property as a string slice.
    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
    }

    /// Returns a custom property as its stored string.
    pub fn get(&self, name: &str) -> Option<&String> {
        self.custom.get(name)
    }

    fn copy_named_from(&mut self, name: &str, source: &Self) {
        match name {
            "appearance" => self.appearance.clone_from(&source.appearance),
            "bg_color" => self.bg_color.clone_from(&source.bg_color),
            "border_color" => self.border_color.clone_from(&source.border_color),
            "border_radius" => self.border_radius.clone_from(&source.border_radius),
            "border_left_width" => self.border_left_width.clone_from(&source.border_left_width),
            "border_right_width" => self
                .border_right_width
                .clone_from(&source.border_right_width),
            "border_top_width" => self.border_top_width.clone_from(&source.border_top_width),
            "border_bottom_width" => self
                .border_bottom_width
                .clone_from(&source.border_bottom_width),
            "font_family" => self.font_family.clone_from(&source.font_family),
            "font_size" => self.font_size.clone_from(&source.font_size),
            "font_weight" => self.font_weight.clone_from(&source.font_weight),
            "font_style" => self.font_style.clone_from(&source.font_style),
            "text_color" => self.text_color.clone_from(&source.text_color),
            "position" => self.position.clone_from(&source.position),
            "left" => self.left.clone_from(&source.left),
            "top" => self.top.clone_from(&source.top),
            "width" => self.width.clone_from(&source.width),
            "height" => self.height.clone_from(&source.height),
            "margin_left" => self.margin_left.clone_from(&source.margin_left),
            "margin_right" => self.margin_right.clone_from(&source.margin_right),
            "margin_top" => self.margin_top.clone_from(&source.margin_top),
            "margin_bottom" => self.margin_bottom.clone_from(&source.margin_bottom),
            "padding_left" => self.padding_left.clone_from(&source.padding_left),
            "padding_right" => self.padding_right.clone_from(&source.padding_right),
            "padding_top" => self.padding_top.clone_from(&source.padding_top),
            "padding_bottom" => self.padding_bottom.clone_from(&source.padding_bottom),
            "flex_grow" => self.flex_grow.clone_from(&source.flex_grow),
            "flex_basis" => self.flex_basis.clone_from(&source.flex_basis),
            "flex_shrink" => self.flex_shrink.clone_from(&source.flex_shrink),
            "align_self" => self.align_self.clone_from(&source.align_self),
            "flex_direction" => self.flex_direction.clone_from(&source.flex_direction),
            "align_items" => self.align_items.clone_from(&source.align_items),
            "justify_content" => self.justify_content.clone_from(&source.justify_content),
            "flex_wrap" => self.flex_wrap.clone_from(&source.flex_wrap),
            "gap" => self.gap.clone_from(&source.gap),
            "transition" => self.transitions.clone_from(&source.transitions),
            _ => {}
        }
    }

    fn apply_global(
        &mut self,
        property: StylePropertyName,
        value: GlobalStyleValue,
        parent: Option<&Self>,
    ) {
        let inherit = match value {
            GlobalStyleValue::Inherit => true,
            GlobalStyleValue::Initial => false,
            GlobalStyleValue::Unset => property.naturally_inherits(),
        };
        let initial = Self::default();
        let source = if inherit {
            parent.unwrap_or(&initial)
        } else {
            &initial
        };
        for name in property.affected_names() {
            self.copy_named_from(name, source);
        }
    }

    fn inherit_unspecified(&mut self, parent: Option<&Self>, specified: &HashSet<String>) {
        let Some(parent) = parent else {
            return;
        };
        for property in [
            StylePropertyName::FontFamily,
            StylePropertyName::FontSize,
            StylePropertyName::FontWeight,
            StylePropertyName::FontStyle,
            StylePropertyName::TextColor,
        ] {
            if !specified.contains(property.name()) {
                self.copy_named_from(property.name(), parent);
            }
        }
    }

    fn apply(&mut self, declaration: StyleDeclaration, parent: Option<&Self>) {
        if let StyleDeclaration::Property(property) = &declaration
            && let Some(value) = property.global()
        {
            self.apply_global(property.property_name(), value, parent);
            return;
        }

        match declaration {
            StyleDeclaration::Property(StyleProperty::Appearance(StyleValue::Value(
                appearance,
            ))) => {
                self.appearance = Some(appearance);
            }
            StyleDeclaration::Property(StyleProperty::BgColor(StyleValue::Value(color))) => {
                self.bg_color = Some(color);
            }
            StyleDeclaration::Property(StyleProperty::BorderColor(StyleValue::Value(color))) => {
                self.border_color = Some(color);
            }
            StyleDeclaration::Property(StyleProperty::BorderRadius(StyleValue::Value(radius))) => {
                self.border_radius = Some(radius);
            }
            StyleDeclaration::Property(StyleProperty::BorderWidth(StyleValue::Value(width))) => {
                self.border_left_width = Some(width);
                self.border_right_width = Some(width);
                self.border_top_width = Some(width);
                self.border_bottom_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderHorizontalWidth(
                StyleValue::Value(width),
            )) => {
                self.border_left_width = Some(width);
                self.border_right_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderVerticalWidth(StyleValue::Value(
                width,
            ))) => {
                self.border_top_width = Some(width);
                self.border_bottom_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderLeftWidth(StyleValue::Value(
                width,
            ))) => {
                self.border_left_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderRightWidth(StyleValue::Value(
                width,
            ))) => {
                self.border_right_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderTopWidth(StyleValue::Value(width))) => {
                self.border_top_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::BorderBottomWidth(StyleValue::Value(
                width,
            ))) => {
                self.border_bottom_width = Some(width);
            }
            StyleDeclaration::Property(StyleProperty::FontFamily(StyleValue::Value(
                font_family,
            ))) => {
                self.font_family = Some(font_family);
            }
            StyleDeclaration::Property(StyleProperty::FontSize(StyleValue::Value(font_size))) => {
                let parent_font_size = parent
                    .and_then(|parent| parent.font_size)
                    .unwrap_or(DEFAULT_ROOT_FONT_SIZE);
                self.font_size = Some(match font_size {
                    Length::Logical(value) => value,
                    Length::Physical(value) => f64::from(value),
                    Length::Em(value) => value * parent_font_size,
                });
            }
            StyleDeclaration::Property(StyleProperty::FontWeight(StyleValue::Value(
                font_weight,
            ))) => {
                self.font_weight = Some(font_weight);
            }
            StyleDeclaration::Property(StyleProperty::FontStyle(StyleValue::Value(font_style))) => {
                self.font_style = Some(font_style);
            }
            StyleDeclaration::Property(StyleProperty::TextColor(StyleValue::Value(color))) => {
                self.text_color = Some(color);
            }
            StyleDeclaration::Property(StyleProperty::Position(StyleValue::Value(position))) => {
                self.position = Some(position);
            }
            StyleDeclaration::Property(StyleProperty::Left(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.left = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Top(StyleValue::Value(length_with_auto))) => {
                self.top = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Width(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.width = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Height(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.height = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Margin(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_left = Some(length_with_auto.clone());
                self.margin_right = Some(length_with_auto.clone());
                self.margin_top = Some(length_with_auto.clone());
                self.margin_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginHorizontal(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_left = Some(length_with_auto.clone());
                self.margin_right = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginVertical(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_top = Some(length_with_auto.clone());
                self.margin_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginLeft(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_left = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginRight(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_right = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginTop(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_top = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::MarginBottom(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.margin_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Padding(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_left = Some(length_with_auto.clone());
                self.padding_right = Some(length_with_auto.clone());
                self.padding_top = Some(length_with_auto.clone());
                self.padding_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingHorizontal(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_left = Some(length_with_auto.clone());
                self.padding_right = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingVertical(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_top = Some(length_with_auto.clone());
                self.padding_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingLeft(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_left = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingRight(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_right = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingTop(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_top = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::PaddingBottom(StyleValue::Value(
                length_with_auto,
            ))) => {
                self.padding_bottom = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::FlexGrow(StyleValue::Value(flex_grow))) => {
                self.flex_grow = Some(flex_grow);
            }
            StyleDeclaration::Property(StyleProperty::FlexBasis(StyleValue::Value(flex_basis))) => {
                self.flex_basis = Some(flex_basis);
            }
            StyleDeclaration::Property(StyleProperty::FlexShrink(StyleValue::Value(
                flex_shrink,
            ))) => {
                self.flex_shrink = Some(flex_shrink);
            }
            StyleDeclaration::Property(StyleProperty::AlignSelf(StyleValue::Value(align_self))) => {
                self.align_self = Some(align_self);
            }
            StyleDeclaration::Property(StyleProperty::FlexDirection(StyleValue::Value(
                flex_direction,
            ))) => {
                self.flex_direction = Some(flex_direction);
            }
            StyleDeclaration::Property(StyleProperty::AlignItems(StyleValue::Value(
                align_items,
            ))) => {
                self.align_items = Some(align_items);
            }
            StyleDeclaration::Property(StyleProperty::JustifyContent(StyleValue::Value(
                justify_content,
            ))) => {
                self.justify_content = Some(justify_content);
            }
            StyleDeclaration::Property(StyleProperty::FlexWrap(StyleValue::Value(flex_wrap))) => {
                self.flex_wrap = Some(flex_wrap);
            }
            StyleDeclaration::Property(StyleProperty::Gap(StyleValue::Value(length_with_auto))) => {
                self.gap = Some(length_with_auto);
            }
            StyleDeclaration::Property(StyleProperty::Transition(StyleValue::Value(
                transitions,
            ))) => {
                self.transitions = transitions;
            }
            StyleDeclaration::Property(_) => {
                unreachable!("global values return before application")
            }
            StyleDeclaration::Custom { name, value } => {
                self.custom.insert(name, value);
            }
        }
    }
}

/// Computes the reactive style matching an element and its class list.
///
/// `default_classes` are combined with the user-supplied classes before
/// selector matching.
pub fn matched_style(
    style_context: Option<Rc<StyleContext>>,
    element: &Element,
    class: PropValue<ClassList>,
    default_classes: &[&'static str],
) -> nestix::Computed<Option<ResolvedStyle>> {
    let default_classes = default_classes.to_vec();
    let class_list = PropValue::from_signal(computed!(
        [class, default_classes] || { class.get().with_defaults(&default_classes) }
    ));
    matched_style_for_class_list(style_context, element, class_list, None)
}

fn matched_style_for_class_list(
    style_context: Option<Rc<StyleContext>>,
    element: &Element,
    class_list: PropValue<ClassList>,
    initial_font_size_override: Option<f64>,
) -> nestix::Computed<Option<ResolvedStyle>> {
    let (placement_version, set_placement_version) = create_state(0);
    let style_sheet = style_context
        .as_ref()
        .and_then(|style_context| style_context.style_sheet.clone());
    let inherited_style = style_context
        .as_ref()
        .map(|style_context| style_context.inherited_style.clone())
        .unwrap_or_else(|| PropValue::from_plain(ResolvedStyle::default()));
    let initial_font_size = initial_font_size_override.unwrap_or_else(|| {
        style_context
            .as_ref()
            .map_or(DEFAULT_ROOT_FONT_SIZE, |style_context| {
                style_context.initial_font_size
            })
    });
    let ancestors = style_context
        .as_ref()
        .map(|style_context| style_context.ancestors.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let ancestor_positions = style_context
        .as_ref()
        .map(|style_context| style_context.ancestor_positions.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let class_registry = style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone());
    let (structure_version, set_structure_version) = style_context
        .as_ref()
        .map(|style_context| {
            (
                style_context.structure_version.clone(),
                style_context.set_structure_version.clone(),
            )
        })
        .unwrap_or_else(|| create_state(0));
    let style_element = element.clone();

    if let Some(class_registry) = &class_registry {
        register_style_element(
            element,
            class_list.clone(),
            class_registry,
            &set_structure_version,
        );
        element.on_place({
            let set_placement_version = set_placement_version.clone();
            move |_| set_placement_version.update(|version| version + 1)
        });
    }

    computed!(
        [
            style_sheet,
            ancestors,
            ancestor_positions,
            class_list,
            placement_version,
            structure_version,
            inherited_style
        ] || {
            let mut inherited_style = inherited_style.get();
            inherited_style.font_size.get_or_insert(initial_font_size);
            let style = if let Some(style_sheet) = &style_sheet {
                placement_version.get();
                let style_sheet = style_sheet.get();
                let uses_structural_position = style_sheet.uses_structural_position();
                if uses_structural_position {
                    structure_version.get();
                }
                let mut context = MatchContext::new(class_list.get())
                    .with_ancestors(ancestors.get())
                    .with_previous_siblings(previous_sibling_class_lists(
                        &style_element,
                        class_registry.as_ref(),
                    ));
                if uses_structural_position {
                    context = context.with_ancestor_positions(ancestor_positions.get());
                    if let Some((index, count)) =
                        logical_child_position(&style_element, class_registry.as_ref())
                    {
                        context = context.with_child_position(index, count);
                    }
                }
                style_sheet.matched_props_with_parent(&context, Some(&inherited_style))
            } else {
                let mut style = ResolvedStyle::default();
                style.inherit_unspecified(Some(&inherited_style), &HashSet::new());
                style
            };

            if style_sheet.is_some() || style != ResolvedStyle::default() {
                Some(style)
            } else {
                None
            }
        }
    )
}

fn scope_ancestors(
    parent_ancestors: PropValue<Vec<ClassList>>,
    class: PropValue<ClassList>,
    default_classes: PropValue<Vec<&'static str>>,
) -> PropValue<Vec<ClassList>> {
    PropValue::from_signal(computed!(
        [parent_ancestors, class, default_classes] || {
            let mut ancestors = Vec::new();
            let default_classes = default_classes.get();
            ancestors.push(class.get().with_defaults(&default_classes));
            ancestors.extend(parent_ancestors.get());
            ancestors
        }
    ))
}

fn scope_ancestor_positions(
    parent_positions: PropValue<Vec<Option<(usize, usize)>>>,
    style_sheet: Option<PropValue<StyleSheet>>,
    element: &Element,
    class_registry: ClassRegistry,
    structure_version: State<usize>,
) -> PropValue<Vec<Option<(usize, usize)>>> {
    let element = element.clone();
    PropValue::from_signal(computed!(
        [parent_positions, style_sheet, structure_version] || {
            let uses_structural_position = style_sheet
                .as_ref()
                .is_some_and(|style_sheet| style_sheet.get().uses_structural_position());
            if !uses_structural_position {
                return Vec::new();
            }
            structure_version.get();
            let mut positions = vec![logical_child_position(&element, Some(&class_registry))];
            positions.extend(parent_positions.get());
            positions
        }
    ))
}

fn inline_or_style<T: Copy + PartialEq>(inline: T, default: T, style: Option<T>) -> T {
    if inline != default {
        inline
    } else {
        style.unwrap_or(inline)
    }
}

/// Resolves an inline auto-or-explicit length against a style-derived value.
///
/// The inline value wins unless it equals `default`.
pub fn style_length_with_auto(
    style: Option<&ResolvedStyle>,
    inline: WithAuto<Length>,
    default: WithAuto<Length>,
    f: impl FnOnce(&ResolvedStyle) -> Option<WithAuto<Length>>,
) -> WithAuto<Length> {
    let value = inline_or_style(inline, default, style.and_then(f));
    match value {
        WithAuto::Auto => WithAuto::Auto,
        WithAuto::Value(length) => WithAuto::Value(
            length.resolve(
                style
                    .and_then(|style| style.font_size)
                    .unwrap_or(DEFAULT_ROOT_FONT_SIZE),
            ),
        ),
    }
}

/// Resolves an inline length against a style-derived value.
///
/// The inline value wins unless it equals `default`.
pub fn style_length(
    style: Option<&ResolvedStyle>,
    inline: Length,
    default: Length,
    f: impl FnOnce(&ResolvedStyle) -> Option<Length>,
) -> Length {
    inline_or_style(inline, default, style.and_then(f)).resolve(
        style
            .and_then(|style| style.font_size)
            .unwrap_or(DEFAULT_ROOT_FONT_SIZE),
    )
}

/// Resolves the effective appearance, preferring a non-default inline value.
pub fn style_appearance(style: Option<&ResolvedStyle>, inline: Appearance) -> Appearance {
    inline_or_style(
        inline,
        Appearance::Native,
        style.and_then(|style| style.appearance),
    )
}

/// Resolves the effective positioning mode.
pub fn style_position(style: Option<&ResolvedStyle>, inline: Position) -> Position {
    inline_or_style(
        inline,
        Position::Relative,
        style.and_then(|style| style.position),
    )
}

/// Resolves the effective flex-grow factor.
pub fn style_flex_grow(style: Option<&ResolvedStyle>, inline: f32) -> f32 {
    inline_or_style(inline, 0.0, style.and_then(|style| style.flex_grow))
}

/// Resolves the effective flex basis.
pub fn style_flex_basis(
    style: Option<&ResolvedStyle>,
    inline: WithAuto<Length>,
) -> WithAuto<Length> {
    style_length_with_auto(style, inline, WithAuto::Auto, |style| {
        style.flex_basis.clone()
    })
}

/// Resolves the effective flex-shrink factor.
pub fn style_flex_shrink(style: Option<&ResolvedStyle>, inline: f32) -> f32 {
    inline_or_style(inline, 1.0, style.and_then(|style| style.flex_shrink))
}

/// Resolves the effective per-item cross-axis alignment.
pub fn style_align_self(style: Option<&ResolvedStyle>, inline: AlignItems) -> AlignItems {
    inline_or_style(
        inline,
        AlignItems::Normal,
        style.and_then(|style| style.align_self),
    )
}

/// Resolves the effective flex main-axis direction.
pub fn style_flex_direction(style: Option<&ResolvedStyle>, inline: FlexDirection) -> FlexDirection {
    inline_or_style(
        inline,
        FlexDirection::Column,
        style.and_then(|style| style.flex_direction),
    )
}

/// Resolves the effective cross-axis alignment for flex children.
pub fn style_align_items(style: Option<&ResolvedStyle>, inline: AlignItems) -> AlignItems {
    inline_or_style(
        inline,
        AlignItems::Normal,
        style.and_then(|style| style.align_items),
    )
}

/// Resolves the effective main-axis distribution for flex children.
pub fn style_justify_content(
    style: Option<&ResolvedStyle>,
    inline: JustifyContent,
) -> JustifyContent {
    inline_or_style(
        inline,
        JustifyContent::Normal,
        style.and_then(|style| style.justify_content),
    )
}

/// Resolves the effective flex wrapping mode.
pub fn style_flex_wrap(style: Option<&ResolvedStyle>, inline: FlexWrap) -> FlexWrap {
    inline_or_style(
        inline,
        FlexWrap::NoWrap,
        style.and_then(|style| style.flex_wrap),
    )
}

/// Resolves the effective gap between flex children.
pub fn style_gap(style: Option<&ResolvedStyle>, inline: WithAuto<Length>) -> WithAuto<Length> {
    style_length_with_auto(style, inline, WithAuto::from(0), |style| style.gap)
}

/// Resolves the effective margin on all four edges.
pub fn style_margin(
    style: Option<&ResolvedStyle>,
    inline: Rect<WithAuto<Length>>,
) -> Rect<WithAuto<Length>> {
    let zero = WithAuto::from(0);
    Rect {
        top: style_length_with_auto(style, inline.top, zero, |style| style.margin_top),
        bottom: style_length_with_auto(style, inline.bottom, zero, |style| style.margin_bottom),
        left: style_length_with_auto(style, inline.left, zero, |style| style.margin_left),
        right: style_length_with_auto(style, inline.right, zero, |style| style.margin_right),
    }
}

/// Resolves the effective padding on all four edges using zero as the default.
pub fn style_padding(
    style: Option<&ResolvedStyle>,
    inline: Rect<WithAuto<Length>>,
) -> Rect<WithAuto<Length>> {
    style_padding_with_default(style, inline, WithAuto::from(0))
}

/// Resolves the effective padding using a caller-provided inline default.
pub fn style_padding_with_default(
    style: Option<&ResolvedStyle>,
    inline: Rect<WithAuto<Length>>,
    default: WithAuto<Length>,
) -> Rect<WithAuto<Length>> {
    Rect {
        top: style_length_with_auto(style, inline.top, default, |style| style.padding_top),
        bottom: style_length_with_auto(style, inline.bottom, default, |style| style.padding_bottom),
        left: style_length_with_auto(style, inline.left, default, |style| style.padding_left),
        right: style_length_with_auto(style, inline.right, default, |style| style.padding_right),
    }
}

/// Combines explicitly supplied font props with inherited resolved values.
pub fn resolve_font_props(
    style: Option<&ResolvedStyle>,
    font_family: Option<String>,
    font_size: Option<Length>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    text_color: Option<Color>,
) -> ResolvedFontProps {
    let inherited = style.map(ResolvedStyle::font).unwrap_or_default();
    ResolvedFontProps {
        font_family: font_family.or(inherited.font_family),
        font_size: font_size
            .map(|font_size| {
                let inherited_size = inherited.font_size.unwrap_or(DEFAULT_ROOT_FONT_SIZE);
                match font_size {
                    Length::Logical(value) => value,
                    Length::Physical(value) => f64::from(value),
                    Length::Em(value) => value * inherited_size,
                }
            })
            .or(inherited.font_size),
        font_weight: font_weight.or(inherited.font_weight),
        font_style: font_style.or(inherited.font_style),
        text_color: text_color.or(inherited.text_color),
    }
}

/// An ordered collection of style rules.
#[derive(Debug, Clone, nestix::InspectableValue)]
pub struct StyleSheet {
    rules: Vec<StyleRule>,
}

impl StyleSheet {
    /// Creates a style sheet from rules in source order.
    pub fn new(rules: Vec<StyleRule>) -> Self {
        Self { rules }
    }

    fn uses_structural_position(&self) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.selector.uses_structural_position())
    }

    /// Resolves declarations matching `context` without a parent style.
    pub fn matched_props(&self, context: &MatchContext) -> ResolvedStyle {
        self.matched_props_with_parent(context, None)
    }

    /// Resolves matching declarations against the parent's final effective
    /// style, including natural inheritance and global values.
    pub fn matched_props_with_parent(
        &self,
        context: &MatchContext,
        parent: Option<&ResolvedStyle>,
    ) -> ResolvedStyle {
        #[derive(Clone)]
        struct Candidate {
            specificity: usize,
            order: usize,
            declaration: StyleDeclaration,
        }

        let mut candidates: HashMap<String, Candidate> = HashMap::new();

        let mut order = 0;
        for rule in &self.rules {
            let Some(specificity) = rule.selector.matched_specificity(context) else {
                order += rule.declarations.len();
                continue;
            };

            for declaration in &rule.declarations {
                let next = Candidate {
                    specificity,
                    order,
                    declaration: declaration.clone(),
                };
                order += 1;

                for name in declaration.affected_names() {
                    let should_replace = candidates.get(name).is_none_or(|previous| {
                        next.specificity > previous.specificity
                            || (next.specificity == previous.specificity
                                && next.order >= previous.order)
                    });

                    if should_replace {
                        candidates.insert(name.to_string(), next.clone());
                    }
                }
            }
        }

        let specified = candidates.keys().cloned().collect::<HashSet<_>>();
        let mut style = ResolvedStyle::default();
        let mut declarations = candidates.into_values().collect::<Vec<_>>();
        declarations.sort_by_key(|candidate| (candidate.specificity, candidate.order));
        for candidate in declarations {
            style.apply(candidate.declaration, parent);
        }
        style.inherit_unspecified(parent, &specified);
        style.resolve_em_lengths();
        style
    }

    /// Returns a style sheet containing this sheet followed by `other`.
    pub fn merged(&self, other: &Self) -> Self {
        let mut style_sheet = self.clone();
        style_sheet.extend(other);
        style_sheet
    }

    /// Adds clones of `other`'s rules after this sheet's rules.
    pub fn extend(&mut self, other: &Self) {
        self.rules.extend(other.rules.clone());
    }

    /// Moves all rules from `other` to the end of this sheet.
    pub fn append(&mut self, other: &mut Self) {
        self.rules.append(&mut other.rules);
    }
}

/// Structural and class information used when matching selectors.
#[derive(Debug, Clone)]
pub struct MatchContext {
    /// Classes on the element being matched.
    pub class_list: ClassList,
    /// Classes on ancestors, ordered from parent to root.
    pub ancestors: Vec<ClassList>,
    /// Classes on preceding siblings, ordered nearest first.
    pub previous_siblings: Vec<ClassList>,
    /// One-based index of the element within its parent.
    pub child_index: Option<usize>,
    /// Total logical child count of the parent.
    pub child_count: Option<usize>,
    ancestor_positions: Vec<Option<(usize, usize)>>,
}

impl MatchContext {
    /// Creates a context for an element with the supplied classes.
    pub fn new(class_list: ClassList) -> Self {
        Self {
            class_list,
            ancestors: Vec::new(),
            previous_siblings: Vec::new(),
            child_index: None,
            child_count: None,
            ancestor_positions: Vec::new(),
        }
    }

    /// Sets ancestor classes, ordered from parent to root.
    pub fn with_ancestors(mut self, ancestors: impl Into<Vec<ClassList>>) -> Self {
        self.ancestors = ancestors.into();
        self.ancestor_positions = vec![None; self.ancestors.len()];
        self
    }

    /// Sets preceding sibling classes, ordered nearest first.
    pub fn with_previous_siblings(mut self, previous_siblings: impl Into<Vec<ClassList>>) -> Self {
        self.previous_siblings = previous_siblings.into();
        self
    }

    /// Sets the element's one-based position and its parent's child count.
    ///
    /// # Panics
    ///
    /// Panics when `index` is zero or greater than `count`.
    pub fn with_child_position(mut self, index: usize, count: usize) -> Self {
        assert!(
            index >= 1 && index <= count,
            "child position must be within 1..=count"
        );
        self.child_index = Some(index);
        self.child_count = Some(count);
        self
    }

    /// Sets one-based structural positions corresponding to [`Self::ancestors`].
    pub fn with_ancestor_positions(
        mut self,
        positions: impl Into<Vec<Option<(usize, usize)>>>,
    ) -> Self {
        self.ancestor_positions = positions.into();
        self.ancestor_positions.resize(self.ancestors.len(), None);
        self
    }

    fn child_position(&self) -> Option<(usize, usize)> {
        Some((self.child_index?, self.child_count?))
    }

    fn parent(&self) -> Option<Self> {
        let parent = self.ancestors.first()?;
        Some(self.ancestor_at(0, parent))
    }

    fn ancestor_at(&self, index: usize, class_list: &ClassList) -> Self {
        let position = self.ancestor_positions.get(index).copied().flatten();
        Self {
            class_list: class_list.clone(),
            ancestors: self.ancestors[index + 1..].to_vec(),
            previous_siblings: Vec::new(),
            child_index: position.map(|(index, _)| index),
            child_count: position.map(|(_, count)| count),
            ancestor_positions: self.ancestor_positions[index + 1..].to_vec(),
        }
    }

    fn previous_sibling(&self) -> Option<Self> {
        let previous_sibling = self.previous_siblings.first()?;
        Some(self.previous_sibling_at(0, previous_sibling))
    }

    fn previous_sibling_at(&self, index: usize, class_list: &ClassList) -> Self {
        Self {
            class_list: class_list.clone(),
            ancestors: self.ancestors.clone(),
            previous_siblings: self.previous_siblings[index + 1..].to_vec(),
            child_index: self
                .child_index
                .and_then(|child_index| child_index.checked_sub(index + 1)),
            child_count: self.child_count,
            ancestor_positions: self.ancestor_positions.clone(),
        }
    }
}

type ClassRegistry = Rc<RefCell<HashMap<Element, PropValue<ClassList>>>>;

/// Styling state inherited by components beneath a style provider or scope.
pub struct StyleContext {
    /// Active style sheet, when styling is enabled.
    pub style_sheet: Option<PropValue<StyleSheet>>,
    /// Reactive ancestor class lists, ordered from parent to root.
    pub ancestors: PropValue<Vec<ClassList>>,
    ancestor_positions: PropValue<Vec<Option<(usize, usize)>>>,
    /// Style values inherited from the parent scope.
    pub inherited_style: PropValue<ResolvedStyle>,
    /// Backend-provided font size used at the root of the style tree.
    pub initial_font_size: f64,
    class_registry: ClassRegistry,
    structure_version: State<usize>,
    set_structure_version: StateSetter<usize>,
}

/// Properties for [`StyleProvider`].
///
/// Generated builder support is an implementation detail of `#[props]`.
#[allow(missing_docs)]
#[props]
pub struct StyleProviderProps {
    /// Style sheet made available to descendant components.
    #[props(start)]
    style_sheet: StyleSheet,
    /// Components that receive the style sheet.
    #[props(default)]
    children: Layout,
}

/// Provides a style sheet to descendant native components.
///
/// When nested, the local rules are appended after inherited rules and
/// therefore win equal-specificity ties.
#[component]
pub fn StyleProvider(props: &StyleProviderProps, element: &Element) -> Element {
    let parent_style_context = element.context::<StyleContext>();
    let style_sheet = if let Some(parent_style_context) = &parent_style_context {
        if let Some(parent_style_sheet) = parent_style_context.style_sheet.clone() {
            PropValue::from_signal(computed!(
                [parent_style_sheet, local: props.style_sheet] || {
                    parent_style_sheet.get().merged(&local.get())
                }
            ))
        } else {
            props.style_sheet.clone()
        }
    } else {
        props.style_sheet.clone()
    };
    let ancestors = parent_style_context
        .as_ref()
        .map(|style_context| style_context.ancestors.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let ancestor_positions = parent_style_context
        .as_ref()
        .map(|style_context| style_context.ancestor_positions.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let class_registry = parent_style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone())
        .unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));
    let (structure_version, set_structure_version) = parent_style_context
        .as_ref()
        .map(|style_context| {
            (
                style_context.structure_version.clone(),
                style_context.set_structure_version.clone(),
            )
        })
        .unwrap_or_else(|| create_state(0));
    let inherited_style = parent_style_context
        .as_ref()
        .map(|style_context| style_context.inherited_style.clone())
        .unwrap_or_else(|| PropValue::from_plain(ResolvedStyle::default()));
    let initial_font_size = parent_style_context
        .as_ref()
        .map_or(DEFAULT_ROOT_FONT_SIZE, |context| context.initial_font_size);

    layout! {
        ContextProvider<StyleContext>(
            StyleContext {
                style_sheet: Some(style_sheet),
                ancestors,
                ancestor_positions,
                inherited_style,
                initial_font_size,
                class_registry,
                structure_version,
                set_structure_version,
            },
        ) {
            $(props.children.clone())
        }
    }
}

/// Properties for [`StyleScope`].
///
/// Generated builder support is an implementation detail of `#[props]`.
#[allow(missing_docs)]
#[props]
pub struct StyleScopeProps {
    /// Classes representing this scope during selector matching.
    #[props(default)]
    class: ClassList,
    /// Renderer-defined classes added to `class`.
    #[props(default)]
    default_classes: Vec<&'static str>,
    /// Effective style inherited by descendant scopes.
    effective_style: Option<ResolvedStyle>,
    /// Overrides the backend initial font size at the root of this scope.
    initial_font_size: Option<f64>,
    /// Components contained by the scope.
    #[props(default)]
    children: Layout,
}

/// Adds an element's classes and effective style to descendant match contexts.
#[cfg_attr(feature = "inspector", component(internal))]
#[cfg_attr(not(feature = "inspector"), component)]
pub fn StyleScope(props: &StyleScopeProps, element: &Element) -> Element {
    let parent_style_context = element.context::<StyleContext>();
    let style_sheet = parent_style_context
        .as_ref()
        .and_then(|style_context| style_context.style_sheet.clone());
    let parent_ancestors = parent_style_context
        .as_ref()
        .map(|style_context| style_context.ancestors.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let parent_ancestor_positions = parent_style_context
        .as_ref()
        .map(|style_context| style_context.ancestor_positions.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let class_registry = parent_style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone())
        .unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));
    let (structure_version, set_structure_version) = parent_style_context
        .as_ref()
        .map(|style_context| {
            (
                style_context.structure_version.clone(),
                style_context.set_structure_version.clone(),
            )
        })
        .unwrap_or_else(|| create_state(0));
    let initial_font_size = props.initial_font_size.get().unwrap_or_else(|| {
        parent_style_context
            .as_ref()
            .map_or(DEFAULT_ROOT_FONT_SIZE, |context| context.initial_font_size)
    });
    let ancestors = scope_ancestors(
        parent_ancestors,
        props.class.clone(),
        props.default_classes.clone(),
    );
    let class_list = PropValue::from_signal(computed!(
        [props.class, props.default_classes] || {
            let default_classes = default_classes.get();
            class.get().with_defaults(&default_classes)
        }
    ));
    let ancestor_positions = scope_ancestor_positions(
        parent_ancestor_positions,
        style_sheet.clone(),
        element,
        class_registry.clone(),
        structure_version.clone(),
    );
    let matched = if props.effective_style.get().is_some() {
        computed!([props.effective_style] || effective_style.get())
    } else {
        matched_style_for_class_list(
            parent_style_context.clone(),
            element,
            class_list.clone(),
            Some(initial_font_size),
        )
    };
    if parent_style_context.is_none() {
        register_style_element(
            element,
            class_list.clone(),
            &class_registry,
            &set_structure_version,
        );
    }
    let inherited_style = PropValue::from_signal(computed!(
        [matched, props.effective_style] || {
            effective_style
                .get()
                .or_else(|| matched.get())
                .unwrap_or_default()
        }
    ));

    layout! {
        ContextProvider<StyleContext>(
            StyleContext {
                style_sheet,
                ancestors,
                ancestor_positions,
                inherited_style,
                initial_font_size,
                class_registry,
                structure_version,
                set_structure_version,
            },
        ) {
            $(props.children.clone())
        }
    }
}

fn previous_sibling_class_lists(
    element: &Element,
    class_registry: Option<&ClassRegistry>,
) -> Vec<ClassList> {
    let Some(class_registry) = class_registry else {
        return Vec::new();
    };
    let previous_siblings = element.previous_siblings();
    let registry = class_registry.borrow();

    previous_siblings
        .into_iter()
        .filter_map(|element| style_class_for_subtree(&element, &registry))
        .map(|class_list| class_list.get())
        .collect()
}

fn register_style_element(
    element: &Element,
    class_list: PropValue<ClassList>,
    class_registry: &ClassRegistry,
    set_structure_version: &StateSetter<usize>,
) {
    class_registry
        .borrow_mut()
        .insert(element.clone(), class_list);

    element.on_unmount({
        let class_registry = class_registry.clone();
        let set_structure_version = set_structure_version.clone();
        let element = element.clone();
        move || {
            class_registry.borrow_mut().remove(&element);
            set_structure_version.update(|version| version + 1);
        }
    });

    element.on_place({
        let set_structure_version = set_structure_version.clone();
        move |_| set_structure_version.update(|version| version + 1)
    });
}

fn logical_child_position(
    element: &Element,
    class_registry: Option<&ClassRegistry>,
) -> Option<(usize, usize)> {
    let class_registry = class_registry?.borrow();
    let mut branch = element.clone();

    loop {
        let parent = branch.parent()?;
        if branch.is_in_list() {
            let siblings = parent.children();
            let mut logical_index = None;
            let mut logical_count = 0;
            for sibling in siblings {
                if style_class_for_subtree(&sibling, &class_registry).is_none() {
                    continue;
                }
                logical_count += 1;
                if sibling == branch {
                    logical_index = Some(logical_count);
                }
            }
            return logical_index.map(|index| (index, logical_count));
        }
        branch = parent;
    }
}

fn style_class_for_subtree(
    element: &Element,
    class_registry: &HashMap<Element, PropValue<ClassList>>,
) -> Option<PropValue<ClassList>> {
    if let Some(class_list) = class_registry.get(element) {
        return Some(class_list.clone());
    }

    element
        .children()
        .into_iter()
        .rev()
        .find_map(|child| style_class_for_subtree(&child, class_registry))
}

/// Converts a [`ResolvedStyle`] entry into a requested output type.
pub trait ResolvedStyleValue: Sized {
    /// Reads a built-in property or parses a custom property named `name`.
    fn from_resolved_style(
        style: &ResolvedStyle,
        name: &str,
        parse: impl FnOnce(&str) -> Option<Self>,
    ) -> Option<Self>;
}

impl ResolvedStyleValue for Color {
    fn from_resolved_style(
        style: &ResolvedStyle,
        name: &str,
        parse: impl FnOnce(&str) -> Option<Self>,
    ) -> Option<Self> {
        match name {
            "bg_color" => style.bg_color,
            "text_color" => style.text_color,
            _ => style.get(name).and_then(|value| parse(value)),
        }
    }
}

/// Resolves an optional inline value against a named resolved style property.
///
/// `inlined` takes precedence. For custom properties, `f` parses the stored
/// string into `T`.
pub fn compute_style<T: ResolvedStyleValue>(
    style_props: Option<&ResolvedStyle>,
    name: &str,
    f: impl FnOnce(&str) -> Option<T>,
    inlined: Option<T>,
) -> Option<T> {
    if inlined.is_some() {
        return inlined;
    }
    let style_props = style_props?;
    T::from_resolved_style(style_props, name, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nestix::{Component, Fragment, FragmentProps, Layout, create_element, mount_root};
    use std::cell::Cell;

    struct Empty;

    impl Component for Empty {
        type Props = ();

        fn on_mount(_: &Element) {}
    }

    fn ancestor_position_recomputation_count(
        selector: StyleSelector,
    ) -> (State<usize>, StateSetter<usize>, Rc<Cell<usize>>) {
        let element = create_element::<Empty>(());
        let class_registry: ClassRegistry = Rc::new(RefCell::new(HashMap::new()));
        let (structure_version, set_structure_version) = create_state(0);
        let positions = scope_ancestor_positions(
            PropValue::from_plain(Vec::new()),
            Some(PropValue::from_plain(StyleSheet::new(vec![StyleRule {
                selector,
                declarations: Vec::new(),
            }]))),
            &element,
            class_registry,
            structure_version.clone(),
        );
        let recomputations = Rc::new(Cell::new(0));
        nestix::effect({
            let recomputations = recomputations.clone();
            move || {
                positions.get();
                recomputations.set(recomputations.get() + 1);
            }
        });
        (structure_version, set_structure_version, recomputations)
    }

    #[test]
    fn ordinary_styles_ignore_unrelated_structure_changes() {
        let (_structure_version, set_structure_version, recomputations) =
            ancestor_position_recomputation_count(StyleSelector::Class("item".to_string()));

        set_structure_version.update(|version| version + 1);

        assert_eq!(recomputations.get(), 1);
    }

    #[test]
    fn structural_styles_track_structure_changes() {
        let (_structure_version, set_structure_version, recomputations) =
            ancestor_position_recomputation_count(StyleSelector::Not(Box::new(
                StyleSelector::FirstChild,
            )));

        set_structure_version.update(|version| version + 1);

        assert_eq!(recomputations.get(), 2);
    }

    #[test]
    fn logical_child_positions_follow_dynamic_list_structure() {
        let first = create_element::<Empty>(());
        let second = create_element::<Empty>(());
        let third = create_element::<Empty>(());
        let registry: ClassRegistry = Rc::new(RefCell::new(HashMap::new()));
        let (structure_version, set_structure_version) = create_state(0);

        for (element, class) in [(&first, "first"), (&second, "second"), (&third, "third")] {
            register_style_element(
                element,
                PropValue::from_plain(ClassList::from(class)),
                &registry,
                &set_structure_version,
            );
        }

        let (children, set_children) = create_state(Layout::from(vec![
            first.clone(),
            second.clone(),
            third.clone(),
        ]));
        let fragment = create_element::<Fragment>(FragmentProps {
            children: PropValue::from_signal(children.clone()),
        });
        mount_root(&fragment);

        assert_eq!(
            logical_child_position(&first, Some(&registry)),
            Some((1, 3))
        );
        assert_eq!(
            logical_child_position(&second, Some(&registry)),
            Some((2, 3))
        );
        assert_eq!(
            logical_child_position(&third, Some(&registry)),
            Some((3, 3))
        );
        let mounted_version = structure_version.get();

        set_children.set_unchecked(Layout::from(vec![second.clone(), first.clone()]));

        assert_eq!(
            logical_child_position(&second, Some(&registry)),
            Some((1, 2))
        );
        assert_eq!(
            logical_child_position(&first, Some(&registry)),
            Some((2, 2))
        );
        assert!(structure_version.get() > mounted_version);
        assert!(!registry.borrow().contains_key(&third));
    }
}
