use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use nestix::{ContextProvider, Element, Layout, PropValue, component, computed, layout, props};

use crate::{AlignItems, Color, Dimension, Rect};

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

    fn specificity(&self) -> usize {
        self.0.len()
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
    Class(ClassList),
    List(Vec<StyleSelector>),
}

impl StyleSelector {
    pub fn matches(&self, context: &MatchContext) -> bool {
        self.matched_specificity(context).is_some()
    }

    pub fn matched_specificity(&self, context: &MatchContext) -> Option<usize> {
        match self {
            StyleSelector::Class(class) if context.class_list.is_superset(class) => {
                Some(class.specificity())
            }
            StyleSelector::Class(_) => None,
            StyleSelector::List(selectors) => selectors
                .iter()
                .filter_map(|selector| selector.matched_specificity(context))
                .max(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StyleProperty {
    BgColor(Color),
    Left(Dimension),
    Top(Dimension),
    Width(Dimension),
    Height(Dimension),
    MarginLeft(Dimension),
    MarginRight(Dimension),
    MarginTop(Dimension),
    MarginBottom(Dimension),
    Grow(f32),
    AlignSelf(AlignItems),
}

impl StyleProperty {
    pub fn name(&self) -> &'static str {
        match self {
            StyleProperty::BgColor(_) => "bg_color",
            StyleProperty::Left(_) => "left",
            StyleProperty::Top(_) => "top",
            StyleProperty::Width(_) => "width",
            StyleProperty::Height(_) => "height",
            StyleProperty::MarginLeft(_) => "margin_left",
            StyleProperty::MarginRight(_) => "margin_right",
            StyleProperty::MarginTop(_) => "margin_top",
            StyleProperty::MarginBottom(_) => "margin_bottom",
            StyleProperty::Grow(_) => "grow",
            StyleProperty::AlignSelf(_) => "align_self",
        }
    }
}

#[derive(Debug, Clone)]
pub enum StyleDeclaration {
    Property(StyleProperty),
    Custom { name: String, value: String },
}

impl StyleDeclaration {
    fn name(&self) -> &str {
        match self {
            StyleDeclaration::Property(property) => property.name(),
            StyleDeclaration::Custom { name, .. } => name,
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
    pub bg_color: Option<Color>,
    pub left: Option<Dimension>,
    pub top: Option<Dimension>,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub margin_left: Option<Dimension>,
    pub margin_right: Option<Dimension>,
    pub margin_top: Option<Dimension>,
    pub margin_bottom: Option<Dimension>,
    pub grow: Option<f32>,
    pub align_self: Option<AlignItems>,
    custom: HashMap<String, String>,
}

impl ResolvedStyle {
    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.custom.get(name)
    }

    fn apply(&mut self, declaration: StyleDeclaration) {
        match declaration {
            StyleDeclaration::Property(StyleProperty::BgColor(color)) => {
                self.bg_color = Some(color);
            }
            StyleDeclaration::Property(StyleProperty::Left(dimension)) => {
                self.left = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Top(dimension)) => {
                self.top = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Width(dimension)) => {
                self.width = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Height(dimension)) => {
                self.height = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginLeft(dimension)) => {
                self.margin_left = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginRight(dimension)) => {
                self.margin_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginTop(dimension)) => {
                self.margin_top = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginBottom(dimension)) => {
                self.margin_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::Grow(grow)) => {
                self.grow = Some(grow);
            }
            StyleDeclaration::Property(StyleProperty::AlignSelf(align_self)) => {
                self.align_self = Some(align_self);
            }
            StyleDeclaration::Custom { name, value } => {
                self.custom.insert(name.clone(), value.clone());
            }
        }
    }
}

pub fn matched_style(
    style_context: Option<Rc<StyleContext>>,
    class: PropValue<ClassList>,
    default_classes: &'static [&'static str],
) -> nestix::Computed<Option<ResolvedStyle>> {
    let style_sheet = style_context.map(|style_context| style_context.style_sheet.clone());
    computed!(
        [style_sheet, class] || {
            style_sheet.as_ref().map(|style_sheet| {
                style_sheet.get().matched_props(&MatchContext {
                    class_list: class.get().with_defaults(default_classes),
                })
            })
        }
    )
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

pub fn style_grow(style: Option<&ResolvedStyle>, inline: f32) -> f32 {
    inline_or_style(inline, 0.0, style.and_then(|style| style.grow))
}

pub fn style_align_self(style: Option<&ResolvedStyle>, inline: AlignItems) -> AlignItems {
    inline_or_style(
        inline,
        AlignItems::Unset,
        style.and_then(|style| style.align_self),
    )
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

#[derive(Debug, Clone)]
pub struct StyleSheet {
    rules: Vec<StyleRule>,
}

impl StyleSheet {
    pub fn new(rules: Vec<StyleRule>) -> Self {
        Self { rules }
    }

    pub fn matched_props(&self, context: &MatchContext) -> ResolvedStyle {
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
                let name = declaration.name().to_string();
                let next = Candidate {
                    specificity,
                    order,
                    declaration: declaration.clone(),
                };
                order += 1;

                let should_replace = candidates.get(&name).is_none_or(|previous| {
                    next.specificity > previous.specificity
                        || (next.specificity == previous.specificity
                            && next.order >= previous.order)
                });

                if should_replace {
                    candidates.insert(name, next);
                }
            }
        }

        let mut style = ResolvedStyle::default();
        let mut declarations = candidates.into_values().collect::<Vec<_>>();
        declarations.sort_by_key(|candidate| (candidate.specificity, candidate.order));
        for candidate in declarations {
            style.apply(candidate.declaration);
        }
        style
    }

    pub fn merged(&self, other: &Self) -> Self {
        let mut rules = self.rules.clone();
        rules.extend(other.rules.clone());
        Self { rules }
    }

    pub fn append(&mut self, other: &mut Self) {
        self.rules.append(&mut other.rules);
    }
}

#[derive(Debug, Clone)]
pub struct MatchContext {
    pub class_list: ClassList,
}

pub struct StyleContext {
    pub style_sheet: PropValue<StyleSheet>,
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
    let style_sheet = if let Some(parent_style_context) = parent_style_context {
        PropValue::from_signal(computed!(
            [parent: parent_style_context.style_sheet, local: props.style_sheet] || {
                parent.get().merged(&local.get())
            }
        ))
    } else {
        props.style_sheet.clone()
    };

    layout! {
        ContextProvider<StyleContext>(StyleContext {style_sheet}) {
            $(props.children.clone())
        }
    }
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
        if name == "bg_color" {
            style.bg_color
        } else {
            style.get(name).and_then(|value| parse(value))
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
