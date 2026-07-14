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
    ContextProvider, Element, Layout, PropValue, State, component, computed, create_state, layout,
    props,
};

use crate::{
    AlignItems, Appearance, Color, Dimension, FlexDirection, FlexWrap, FontStyle, FontWeight,
    JustifyContent, Rect, ResolvedFontProps,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassList(HashSet<String>);

impl ClassList {
    pub fn contains(&self, class: &str) -> bool {
        self.0.contains(class)
    }

    pub fn is_superset(&self, other: &ClassList) -> bool {
        self.0.is_superset(&other.0)
    }

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

#[derive(Debug, Clone)]
pub enum StyleSelector {
    Class(String),
    Not(Box<StyleSelector>),
    All(Vec<StyleSelector>),
    Child {
        parent: Box<StyleSelector>,
        child: Box<StyleSelector>,
    },
    Descendant {
        ancestor: Box<StyleSelector>,
        descendant: Box<StyleSelector>,
    },
    AdjacentSibling {
        previous: Box<StyleSelector>,
        sibling: Box<StyleSelector>,
    },
    SubsequentSibling {
        previous: Box<StyleSelector>,
        sibling: Box<StyleSelector>,
    },
    List(Vec<StyleSelector>),
}

impl StyleSelector {
    pub fn matches(&self, context: &MatchContext) -> bool {
        self.matched_specificity(context).is_some()
    }

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
}

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
    /// Font family name. This property is inherited.
    ///
    /// **Available value**: a single-word family name (`Arial`), a double-quoted
    /// family name (`"Comic Sans"`).
    FontFamily(StyleValue<String>),
    /// Font size in logical pixels. This property is inherited.
    ///
    /// **Available value**: a pixel value such as `14px`.
    FontSize(StyleValue<f64>),
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
    /// Horizontal position offset from the left edge of the containing block.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Left(StyleValue<Dimension>),
    /// Vertical position offset from the top edge of the containing block.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Top(StyleValue<Dimension>),
    /// Preferred layout width.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Width(StyleValue<Dimension>),
    /// Preferred layout height.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Height(StyleValue<Dimension>),
    /// Margin applied to all four edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Margin(StyleValue<Dimension>),
    /// Margin applied to the left and right edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginHorizontal(StyleValue<Dimension>),
    /// Margin applied to the top and bottom edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginVertical(StyleValue<Dimension>),
    /// Margin applied to the left edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginLeft(StyleValue<Dimension>),
    /// Margin applied to the right edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginRight(StyleValue<Dimension>),
    /// Margin applied to the top edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginTop(StyleValue<Dimension>),
    /// Margin applied to the bottom edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginBottom(StyleValue<Dimension>),
    /// Padding applied to all four edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Padding(StyleValue<Dimension>),
    /// Padding applied to the left and right edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingHorizontal(StyleValue<Dimension>),
    /// Padding applied to the top and bottom edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingVertical(StyleValue<Dimension>),
    /// Padding applied to the left edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingLeft(StyleValue<Dimension>),
    /// Padding applied to the right edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingRight(StyleValue<Dimension>),
    /// Padding applied to the top edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingTop(StyleValue<Dimension>),
    /// Padding applied to the bottom edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    PaddingBottom(StyleValue<Dimension>),
    /// Flex grow factor used when distributing free space.
    ///
    /// **Available value**: a number.
    FlexGrow(StyleValue<f32>),
    /// Initial main size of the flex item.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    FlexBasis(StyleValue<Dimension>),
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
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Gap(StyleValue<Dimension>),
}

impl StyleProperty {
    pub(crate) fn property_name(&self) -> StylePropertyName {
        match self {
            Self::Appearance(_) => StylePropertyName::Appearance,
            Self::BgColor(_) => StylePropertyName::BgColor,
            Self::FontFamily(_) => StylePropertyName::FontFamily,
            Self::FontSize(_) => StylePropertyName::FontSize,
            Self::FontWeight(_) => StylePropertyName::FontWeight,
            Self::FontStyle(_) => StylePropertyName::FontStyle,
            Self::TextColor(_) => StylePropertyName::TextColor,
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
        }
    }

    pub fn name(&self) -> &'static str {
        self.property_name().name()
    }

    fn global(&self) -> Option<GlobalStyleValue> {
        match self {
            Self::Appearance(value) => value.global(),
            Self::BgColor(value) => value.global(),
            Self::FontFamily(value) => value.global(),
            Self::FontSize(value) => value.global(),
            Self::FontWeight(value) => value.global(),
            Self::FontStyle(value) => value.global(),
            Self::TextColor(value) => value.global(),
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
        }
    }
}

#[derive(Debug, Clone)]
pub enum StyleDeclaration {
    Property(StyleProperty),
    Custom { name: String, value: String },
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

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: StyleSelector,
    pub declarations: Vec<StyleDeclaration>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedStyle {
    pub appearance: Option<Appearance>,
    pub bg_color: Option<Color>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub text_color: Option<Color>,
    pub left: Option<Dimension>,
    pub top: Option<Dimension>,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub margin_left: Option<Dimension>,
    pub margin_right: Option<Dimension>,
    pub margin_top: Option<Dimension>,
    pub margin_bottom: Option<Dimension>,
    pub padding_left: Option<Dimension>,
    pub padding_right: Option<Dimension>,
    pub padding_top: Option<Dimension>,
    pub padding_bottom: Option<Dimension>,
    pub flex_grow: Option<f32>,
    pub flex_basis: Option<Dimension>,
    pub flex_shrink: Option<f32>,
    pub align_self: Option<AlignItems>,
    pub flex_direction: Option<FlexDirection>,
    pub align_items: Option<AlignItems>,
    pub justify_content: Option<JustifyContent>,
    pub flex_wrap: Option<FlexWrap>,
    pub gap: Option<Dimension>,
    custom: HashMap<String, String>,
}

impl ResolvedStyle {
    pub fn font(&self) -> ResolvedFontProps {
        ResolvedFontProps {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
            text_color: self.text_color,
        }
    }

    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.custom.get(name)
    }

    fn copy_named_from(&mut self, name: &str, source: &Self) {
        match name {
            "appearance" => self.appearance.clone_from(&source.appearance),
            "bg_color" => self.bg_color.clone_from(&source.bg_color),
            "font_family" => self.font_family.clone_from(&source.font_family),
            "font_size" => self.font_size.clone_from(&source.font_size),
            "font_weight" => self.font_weight.clone_from(&source.font_weight),
            "font_style" => self.font_style.clone_from(&source.font_style),
            "text_color" => self.text_color.clone_from(&source.text_color),
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
            StyleDeclaration::Property(StyleProperty::FontFamily(StyleValue::Value(
                font_family,
            ))) => {
                self.font_family = Some(font_family);
            }
            StyleDeclaration::Property(StyleProperty::FontSize(StyleValue::Value(font_size))) => {
                self.font_size = Some(font_size);
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
            StyleDeclaration::Property(StyleProperty::Left(StyleValue::Value(dimension))) => {
                self.left = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Top(StyleValue::Value(dimension))) => {
                self.top = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Width(StyleValue::Value(dimension))) => {
                self.width = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Height(StyleValue::Value(dimension))) => {
                self.height = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Margin(StyleValue::Value(dimension))) => {
                self.margin_left = Some(dimension.clone());
                self.margin_right = Some(dimension.clone());
                self.margin_top = Some(dimension.clone());
                self.margin_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginHorizontal(StyleValue::Value(
                dimension,
            ))) => {
                self.margin_left = Some(dimension.clone());
                self.margin_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginVertical(StyleValue::Value(
                dimension,
            ))) => {
                self.margin_top = Some(dimension.clone());
                self.margin_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginLeft(StyleValue::Value(dimension))) => {
                self.margin_left = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginRight(StyleValue::Value(
                dimension,
            ))) => {
                self.margin_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginTop(StyleValue::Value(dimension))) => {
                self.margin_top = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginBottom(StyleValue::Value(
                dimension,
            ))) => {
                self.margin_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Padding(StyleValue::Value(dimension))) => {
                self.padding_left = Some(dimension.clone());
                self.padding_right = Some(dimension.clone());
                self.padding_top = Some(dimension.clone());
                self.padding_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingHorizontal(StyleValue::Value(
                dimension,
            ))) => {
                self.padding_left = Some(dimension.clone());
                self.padding_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingVertical(StyleValue::Value(
                dimension,
            ))) => {
                self.padding_top = Some(dimension.clone());
                self.padding_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingLeft(StyleValue::Value(
                dimension,
            ))) => {
                self.padding_left = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingRight(StyleValue::Value(
                dimension,
            ))) => {
                self.padding_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingTop(StyleValue::Value(dimension))) => {
                self.padding_top = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::PaddingBottom(StyleValue::Value(
                dimension,
            ))) => {
                self.padding_bottom = Some(dimension);
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
            StyleDeclaration::Property(StyleProperty::Gap(StyleValue::Value(dimension))) => {
                self.gap = Some(dimension);
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
    matched_style_for_class_list(style_context, element, class_list)
}

fn matched_style_for_class_list(
    style_context: Option<Rc<StyleContext>>,
    element: &Element,
    class_list: PropValue<ClassList>,
) -> nestix::Computed<Option<ResolvedStyle>> {
    let placement_version: State<usize> = create_state(0);

    let style_sheet = style_context
        .as_ref()
        .and_then(|style_context| style_context.style_sheet.clone());
    let inherited_style = style_context
        .as_ref()
        .map(|style_context| style_context.inherited_style.clone())
        .unwrap_or_else(|| PropValue::from_plain(ResolvedStyle::default()));
    let ancestors = style_context
        .as_ref()
        .map(|style_context| style_context.ancestors.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let class_registry = style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone());
    let style_element = element.clone();

    if let Some(class_registry) = &class_registry {
        class_registry
            .borrow_mut()
            .insert(element.clone(), class_list.clone());

        element.on_unmount({
            let class_registry = class_registry.clone();
            let element = element.clone();
            move || {
                class_registry.borrow_mut().remove(&element);
            }
        });

        element.on_place({
            let placement_version = placement_version.clone();
            move |_| {
                placement_version.update(|version| version + 1);
            }
        });
    }

    computed!(
        [
            style_sheet,
            ancestors,
            class_list,
            placement_version,
            inherited_style
        ] || {
            placement_version.get();
            let inherited_style = inherited_style.get();
            let style = if let Some(style_sheet) = &style_sheet {
                style_sheet.get().matched_props_with_parent(
                    &MatchContext::new(class_list.get())
                        .with_ancestors(ancestors.get())
                        .with_previous_siblings(previous_sibling_class_lists(
                            &style_element,
                            class_registry.as_ref(),
                        )),
                    Some(&inherited_style),
                )
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

fn inline_or_style<T: Copy + PartialEq>(inline: T, default: T, style: Option<T>) -> T {
    if inline != default {
        inline
    } else {
        style.unwrap_or(inline)
    }
}

pub fn style_dimension(
    style: Option<&ResolvedStyle>,
    inline: Dimension,
    default: Dimension,
    f: impl FnOnce(&ResolvedStyle) -> Option<Dimension>,
) -> Dimension {
    inline_or_style(inline, default, style.and_then(f))
}

pub fn style_appearance(style: Option<&ResolvedStyle>, inline: Appearance) -> Appearance {
    inline_or_style(
        inline,
        Appearance::Native,
        style.and_then(|style| style.appearance),
    )
}

pub fn style_flex_grow(style: Option<&ResolvedStyle>, inline: f32) -> f32 {
    inline_or_style(inline, 0.0, style.and_then(|style| style.flex_grow))
}

pub fn style_flex_basis(style: Option<&ResolvedStyle>, inline: Dimension) -> Dimension {
    style_dimension(style, inline, Dimension::Auto, |style| {
        style.flex_basis.clone()
    })
}

pub fn style_flex_shrink(style: Option<&ResolvedStyle>, inline: f32) -> f32 {
    inline_or_style(inline, 1.0, style.and_then(|style| style.flex_shrink))
}

pub fn style_align_self(style: Option<&ResolvedStyle>, inline: AlignItems) -> AlignItems {
    inline_or_style(
        inline,
        AlignItems::Normal,
        style.and_then(|style| style.align_self),
    )
}

pub fn style_flex_direction(style: Option<&ResolvedStyle>, inline: FlexDirection) -> FlexDirection {
    inline_or_style(
        inline,
        FlexDirection::Column,
        style.and_then(|style| style.flex_direction),
    )
}

pub fn style_align_items(style: Option<&ResolvedStyle>, inline: AlignItems) -> AlignItems {
    inline_or_style(
        inline,
        AlignItems::Normal,
        style.and_then(|style| style.align_items),
    )
}

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

pub fn style_flex_wrap(style: Option<&ResolvedStyle>, inline: FlexWrap) -> FlexWrap {
    inline_or_style(
        inline,
        FlexWrap::NoWrap,
        style.and_then(|style| style.flex_wrap),
    )
}

pub fn style_gap(style: Option<&ResolvedStyle>, inline: Dimension) -> Dimension {
    style_dimension(style, inline, Dimension::from(0), |style| style.gap)
}

pub fn style_margin(style: Option<&ResolvedStyle>, inline: Rect<Dimension>) -> Rect<Dimension> {
    let zero = Dimension::from(0);
    Rect {
        top: style_dimension(style, inline.top, zero, |style| style.margin_top),
        bottom: style_dimension(style, inline.bottom, zero, |style| style.margin_bottom),
        left: style_dimension(style, inline.left, zero, |style| style.margin_left),
        right: style_dimension(style, inline.right, zero, |style| style.margin_right),
    }
}

pub fn style_padding(style: Option<&ResolvedStyle>, inline: Rect<Dimension>) -> Rect<Dimension> {
    style_padding_with_default(style, inline, Dimension::from(0))
}

pub fn style_padding_with_default(
    style: Option<&ResolvedStyle>,
    inline: Rect<Dimension>,
    default: Dimension,
) -> Rect<Dimension> {
    Rect {
        top: style_dimension(style, inline.top, default, |style| style.padding_top),
        bottom: style_dimension(style, inline.bottom, default, |style| style.padding_bottom),
        left: style_dimension(style, inline.left, default, |style| style.padding_left),
        right: style_dimension(style, inline.right, default, |style| style.padding_right),
    }
}

pub fn resolve_font_props(
    style: Option<&ResolvedStyle>,
    font_family: Option<String>,
    font_size: Option<f64>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    text_color: Option<Color>,
) -> ResolvedFontProps {
    let inherited = style.map(ResolvedStyle::font).unwrap_or_default();
    ResolvedFontProps {
        font_family: font_family.or(inherited.font_family),
        font_size: font_size.or(inherited.font_size),
        font_weight: font_weight.or(inherited.font_weight),
        font_style: font_style.or(inherited.font_style),
        text_color: text_color.or(inherited.text_color),
    }
}

#[derive(Debug, Clone)]
pub struct StyleSheet {
    rules: Vec<StyleRule>,
}

impl StyleSheet {
    pub fn new(rules: Vec<StyleRule>) -> Self {
        Self { rules }
    }

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
        style
    }

    pub fn merged(&self, other: &Self) -> Self {
        let mut style_sheet = self.clone();
        style_sheet.extend(other);
        style_sheet
    }

    pub fn extend(&mut self, other: &Self) {
        self.rules.extend(other.rules.clone());
    }

    pub fn append(&mut self, other: &mut Self) {
        self.rules.append(&mut other.rules);
    }
}

#[derive(Debug, Clone)]
pub struct MatchContext {
    pub class_list: ClassList,
    pub ancestors: Vec<ClassList>,
    pub previous_siblings: Vec<ClassList>,
}

impl MatchContext {
    pub fn new(class_list: ClassList) -> Self {
        Self {
            class_list,
            ancestors: Vec::new(),
            previous_siblings: Vec::new(),
        }
    }

    pub fn with_ancestors(mut self, ancestors: impl Into<Vec<ClassList>>) -> Self {
        self.ancestors = ancestors.into();
        self
    }

    pub fn with_previous_siblings(mut self, previous_siblings: impl Into<Vec<ClassList>>) -> Self {
        self.previous_siblings = previous_siblings.into();
        self
    }

    fn parent(&self) -> Option<Self> {
        let parent = self.ancestors.first()?;
        Some(self.ancestor_at(0, parent))
    }

    fn ancestor_at(&self, index: usize, class_list: &ClassList) -> Self {
        Self {
            class_list: class_list.clone(),
            ancestors: self.ancestors[index + 1..].to_vec(),
            previous_siblings: Vec::new(),
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
        }
    }
}

type ClassRegistry = Rc<RefCell<HashMap<Element, PropValue<ClassList>>>>;

pub struct StyleContext {
    pub style_sheet: Option<PropValue<StyleSheet>>,
    pub ancestors: PropValue<Vec<ClassList>>,
    pub inherited_style: PropValue<ResolvedStyle>,
    class_registry: ClassRegistry,
}

#[props]
pub struct StyleProviderProps {
    #[props(start)]
    style_sheet: StyleSheet,
    #[props(default)]
    children: Layout,
}

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
    let class_registry = parent_style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone())
        .unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));
    let inherited_style = parent_style_context
        .map(|style_context| style_context.inherited_style.clone())
        .unwrap_or_else(|| PropValue::from_plain(ResolvedStyle::default()));

    layout! {
        ContextProvider<StyleContext>(StyleContext {style_sheet: Some(style_sheet), ancestors, inherited_style, class_registry}) {
            $(props.children.clone())
        }
    }
}

#[props]
pub struct StyleScopeProps {
    #[props(default)]
    class: ClassList,
    #[props(default)]
    default_classes: Vec<&'static str>,
    effective_style: Option<ResolvedStyle>,
    #[props(default)]
    children: Layout,
}

#[component]
pub fn StyleScope(props: &StyleScopeProps, element: &Element) -> Element {
    let parent_style_context = element.context::<StyleContext>();
    let style_sheet = parent_style_context
        .as_ref()
        .and_then(|style_context| style_context.style_sheet.clone());
    let parent_ancestors = parent_style_context
        .as_ref()
        .map(|style_context| style_context.ancestors.clone())
        .unwrap_or_else(|| PropValue::from_plain(Vec::new()));
    let class_registry = parent_style_context
        .as_ref()
        .map(|style_context| style_context.class_registry.clone())
        .unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));
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
    let matched = if props.effective_style.get().is_some() {
        computed!([props.effective_style] || effective_style.get())
    } else {
        matched_style_for_class_list(parent_style_context.clone(), element, class_list.clone())
    };
    if parent_style_context.is_none() {
        class_registry
            .borrow_mut()
            .insert(element.clone(), class_list.clone());
        element.on_unmount({
            let class_registry = class_registry.clone();
            let element = element.clone();
            move || {
                class_registry.borrow_mut().remove(&element);
            }
        });
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
        ContextProvider<StyleContext>(StyleContext {style_sheet, ancestors, inherited_style, class_registry}) {
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

pub trait ResolvedStyleValue: Sized {
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
