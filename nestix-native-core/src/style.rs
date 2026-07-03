use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use nestix::{
    ContextProvider, Element, Layout, PropValue, State, component, computed, create_state, layout,
    props,
};

use crate::{AlignItems, Color, Dimension, FlexDirection, FlexWrap, Rect};

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
    /// Background color applied to the element.
    ///
    /// **Available value**: a named color (`white`, `black`, `transparent`, `red`,
    /// `green`, `blue`), or a 6/8 digit hex color (`#RRGGBB` or `#RRGGBBAA`).
    BgColor(Color),
    /// Horizontal position offset from the left edge of the containing block.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Left(Dimension),
    /// Vertical position offset from the top edge of the containing block.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Top(Dimension),
    /// Preferred layout width.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Width(Dimension),
    /// Preferred layout height.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Height(Dimension),
    /// Margin applied to all four edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    Margin(Dimension),
    /// Margin applied to the left and right edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginHorizontal(Dimension),
    /// Margin applied to the top and bottom edges.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginVertical(Dimension),
    /// Margin applied to the left edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginLeft(Dimension),
    /// Margin applied to the right edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginRight(Dimension),
    /// Margin applied to the top edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginTop(Dimension),
    /// Margin applied to the bottom edge.
    ///
    /// **Available value**: `auto`, or a pixel value such as `30px`.
    MarginBottom(Dimension),
    /// Flex grow factor used when distributing free space.
    ///
    /// **Available value**: a number.
    Grow(f32),
    /// Cross-axis alignment override for this element within its flex parent.
    ///
    /// **Available value**: `unset`, `start`, `end`, `flex-start`, `flex-end`,
    /// `center`, `baseline`, or `stretch`.
    AlignSelf(AlignItems),
    /// Main-axis direction for this element's flex children.
    ///
    /// **Available value**: `row`, `row-reverse`, `column`, or `column-reverse`.
    FlexDirection(FlexDirection),
    /// Cross-axis alignment for this element's flex children.
    ///
    /// **Available value**: `unset`, `start`, `end`, `flex-start`, `flex-end`,
    /// `center`, `baseline`, or `stretch`.
    AlignItems(AlignItems),
    /// Wrapping behavior for this element's flex children.
    ///
    /// **Available value**: `nowrap`, `no-wrap`, or `wrap`.
    FlexWrap(FlexWrap),
}

impl StyleProperty {
    pub fn name(&self) -> &'static str {
        match self {
            StyleProperty::BgColor(_) => "bg_color",
            StyleProperty::Left(_) => "left",
            StyleProperty::Top(_) => "top",
            StyleProperty::Width(_) => "width",
            StyleProperty::Height(_) => "height",
            StyleProperty::Margin(_) => "margin",
            StyleProperty::MarginHorizontal(_) => "margin_horizontal",
            StyleProperty::MarginVertical(_) => "margin_vertical",
            StyleProperty::MarginLeft(_) => "margin_left",
            StyleProperty::MarginRight(_) => "margin_right",
            StyleProperty::MarginTop(_) => "margin_top",
            StyleProperty::MarginBottom(_) => "margin_bottom",
            StyleProperty::Grow(_) => "grow",
            StyleProperty::AlignSelf(_) => "align_self",
            StyleProperty::FlexDirection(_) => "flex_direction",
            StyleProperty::AlignItems(_) => "align_items",
            StyleProperty::FlexWrap(_) => "flex_wrap",
        }
    }

    fn affected_names(&self) -> &'static [&'static str] {
        match self {
            StyleProperty::BgColor(_) => &["bg_color"],
            StyleProperty::Left(_) => &["left"],
            StyleProperty::Top(_) => &["top"],
            StyleProperty::Width(_) => &["width"],
            StyleProperty::Height(_) => &["height"],
            StyleProperty::Margin(_) => {
                &["margin_left", "margin_right", "margin_top", "margin_bottom"]
            }
            StyleProperty::MarginHorizontal(_) => &["margin_left", "margin_right"],
            StyleProperty::MarginVertical(_) => &["margin_top", "margin_bottom"],
            StyleProperty::MarginLeft(_) => &["margin_left"],
            StyleProperty::MarginRight(_) => &["margin_right"],
            StyleProperty::MarginTop(_) => &["margin_top"],
            StyleProperty::MarginBottom(_) => &["margin_bottom"],
            StyleProperty::Grow(_) => &["grow"],
            StyleProperty::AlignSelf(_) => &["align_self"],
            StyleProperty::FlexDirection(_) => &["flex_direction"],
            StyleProperty::AlignItems(_) => &["align_items"],
            StyleProperty::FlexWrap(_) => &["flex_wrap"],
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
            StyleDeclaration::Property(property) => property.affected_names().to_vec(),
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
    pub flex_direction: Option<FlexDirection>,
    pub align_items: Option<AlignItems>,
    pub flex_wrap: Option<FlexWrap>,
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
            StyleDeclaration::Property(StyleProperty::Margin(dimension)) => {
                self.margin_left = Some(dimension.clone());
                self.margin_right = Some(dimension.clone());
                self.margin_top = Some(dimension.clone());
                self.margin_bottom = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginHorizontal(dimension)) => {
                self.margin_left = Some(dimension.clone());
                self.margin_right = Some(dimension);
            }
            StyleDeclaration::Property(StyleProperty::MarginVertical(dimension)) => {
                self.margin_top = Some(dimension.clone());
                self.margin_bottom = Some(dimension);
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
            StyleDeclaration::Property(StyleProperty::FlexDirection(flex_direction)) => {
                self.flex_direction = Some(flex_direction);
            }
            StyleDeclaration::Property(StyleProperty::AlignItems(align_items)) => {
                self.align_items = Some(align_items);
            }
            StyleDeclaration::Property(StyleProperty::FlexWrap(flex_wrap)) => {
                self.flex_wrap = Some(flex_wrap);
            }
            StyleDeclaration::Custom { name, value } => {
                self.custom.insert(name.clone(), value.clone());
            }
        }
    }
}

pub fn matched_style(
    style_context: Option<Rc<StyleContext>>,
    element: &Element,
    class: PropValue<ClassList>,
    default_classes: &'static [&'static str],
) -> nestix::Computed<Option<ResolvedStyle>> {
    let class_list = PropValue::from_signal(computed!(
        [class] || { class.get().with_defaults(default_classes) }
    ));
    let placement_version: State<usize> = create_state(0);

    let style_sheet = style_context
        .as_ref()
        .and_then(|style_context| style_context.style_sheet.clone());
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
        [style_sheet, ancestors, class_list, placement_version] || {
            style_sheet.as_ref().map(|style_sheet| {
                placement_version.get();
                style_sheet.get().matched_props(
                    &MatchContext::new(class_list.get())
                        .with_ancestors(ancestors.get())
                        .with_previous_siblings(previous_sibling_class_lists(
                            &style_element,
                            class_registry.as_ref(),
                        )),
                )
            })
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
        AlignItems::Unset,
        style.and_then(|style| style.align_items),
    )
}

pub fn style_flex_wrap(style: Option<&ResolvedStyle>, inline: FlexWrap) -> FlexWrap {
    inline_or_style(
        inline,
        FlexWrap::NoWrap,
        style.and_then(|style| style.flex_wrap),
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

        let mut style = ResolvedStyle::default();
        let mut declarations = candidates.into_values().collect::<Vec<_>>();
        declarations.sort_by_key(|candidate| (candidate.specificity, candidate.order));
        for candidate in declarations {
            style.apply(candidate.declaration);
        }
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
        .map(|style_context| style_context.class_registry.clone())
        .unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));

    layout! {
        ContextProvider<StyleContext>(StyleContext {style_sheet: Some(style_sheet), ancestors, class_registry}) {
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

    layout! {
        ContextProvider<StyleContext>(StyleContext {style_sheet, ancestors, class_registry}) {
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
