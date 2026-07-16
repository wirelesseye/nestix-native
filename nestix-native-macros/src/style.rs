use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{
    Error, Expr, Ident, Result, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

use crate::utils::nestix_native_path;

pub fn style(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as StyleSheetInput);
    expand_style(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct StyleSheetInput {
    items: Vec<StyleItemInput>,
}

impl Parse for StyleSheetInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self { items })
    }
}

enum StyleItemInput {
    Rule(StyleRuleInput),
    Inserted(Expr),
}

impl Parse for StyleItemInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            let content;
            parenthesized!(content in input);
            return Ok(Self::Inserted(content.parse()?));
        }

        Ok(Self::Rule(input.parse()?))
    }
}

struct StyleRuleInput {
    selector: SelectorInput,
    props: Vec<StylePropInput>,
    nested_rules: Vec<StyleRuleInput>,
}

impl Parse for StyleRuleInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;

        let content;
        braced!(content in input);

        let mut props = Vec::new();
        let mut nested_rules = Vec::new();
        while !content.is_empty() {
            if content.peek(Ident) || content.peek(Token![-]) {
                props.push(content.parse()?);
            } else {
                nested_rules.push(content.parse()?);
            }
        }

        Ok(Self {
            selector,
            props,
            nested_rules,
        })
    }
}

#[derive(Clone)]
struct SelectorInput {
    selectors: Vec<SelectorAst>,
}

#[derive(Clone)]
enum SelectorAst {
    Class(String),
    Not(SelectorInput),
    FirstChild,
    LastChild,
    NthChild { a: isize, b: isize },
    All(Vec<SelectorAst>),
    Child(Box<SelectorAst>, Box<SelectorAst>),
    Descendant(Box<SelectorAst>, Box<SelectorAst>),
    AdjacentSibling(Box<SelectorAst>, Box<SelectorAst>),
    SubsequentSibling(Box<SelectorAst>, Box<SelectorAst>),
    Parent(ParentSelectorInput),
}

#[derive(Clone, Copy)]
enum ParentSelectorInput {
    Explicit(Span),
    Relative(Span),
}

impl Parse for SelectorInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut selectors = Vec::new();

        loop {
            selectors.push(parse_selector_chain(input)?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self { selectors })
    }
}

fn parse_selector_chain(input: ParseStream<'_>) -> Result<SelectorAst> {
    let mut selector = if input.peek(Token![>]) || input.peek(Token![+]) || input.peek(Token![~]) {
        SelectorAst::Parent(ParentSelectorInput::Relative(input.span()))
    } else {
        parse_compound_selector(input)?
    };

    loop {
        let next_selector = if input.peek(Token![>]) {
            input.parse::<Token![>]>()?;
            if input.peek(Token![>]) {
                input.parse::<Token![>]>()?;
                let next = parse_compound_selector(input)?;
                SelectorAst::Descendant(Box::new(selector), Box::new(next))
            } else {
                let next = parse_compound_selector(input)?;
                SelectorAst::Child(Box::new(selector), Box::new(next))
            }
        } else if input.peek(Token![+]) {
            input.parse::<Token![+]>()?;
            let next = parse_compound_selector(input)?;
            SelectorAst::AdjacentSibling(Box::new(selector), Box::new(next))
        } else if input.peek(Token![~]) {
            input.parse::<Token![~]>()?;
            let next = parse_compound_selector(input)?;
            SelectorAst::SubsequentSibling(Box::new(selector), Box::new(next))
        } else {
            break;
        };

        selector = next_selector;
    }

    Ok(selector)
}

fn parse_compound_selector(input: ParseStream<'_>) -> Result<SelectorAst> {
    let mut selectors = Vec::new();

    loop {
        if input.peek(Token![.]) {
            selectors.push(SelectorAst::Class(parse_class(input)?));
        } else if input.peek(Token![:]) {
            selectors.push(parse_pseudo_selector(input)?);
        } else if input.peek(Token![&]) {
            let parent: Token![&] = input.parse()?;
            if input.peek(Token![-]) {
                return Err(Error::new(
                    parent.span,
                    "SCSS parent-selector interpolation such as `&-suffix` is not supported",
                ));
            }
            selectors.push(SelectorAst::Parent(ParentSelectorInput::Explicit(
                parent.span,
            )));
        } else {
            break;
        }
    }

    match selectors.len() {
        0 => Err(input.error("expected selector")),
        1 => Ok(selectors.pop().unwrap()),
        _ => Ok(SelectorAst::All(selectors)),
    }
}

fn parse_class(input: ParseStream<'_>) -> Result<String> {
    input.parse::<Token![.]>()?;
    let class_name: Ident = input.parse()?;
    Ok(class_name.to_string())
}

fn parse_pseudo_selector(input: ParseStream<'_>) -> Result<SelectorAst> {
    input.parse::<Token![:]>()?;
    let name: Ident = input.parse()?;

    match name.to_string().as_str() {
        "not" => {
            let content;
            parenthesized!(content in input);
            Ok(SelectorAst::Not(content.parse()?))
        }
        "first_child" => Ok(SelectorAst::FirstChild),
        "last_child" => Ok(SelectorAst::LastChild),
        "nth_child" => {
            let content;
            parenthesized!(content in input);
            let (a, b) = parse_nth_child_formula(&content)?;
            Ok(SelectorAst::NthChild { a, b })
        }
        _ => Err(Error::new_spanned(
            name,
            "unsupported pseudo selector; expected `not`, `first_child`, `last_child`, or `nth_child`",
        )),
    }
}

fn parse_nth_child_formula(input: ParseStream<'_>) -> Result<(isize, isize)> {
    if input.is_empty() {
        return Err(input.error("expected an `An+B` expression in `nth_child()`"));
    }

    let tokens: TokenStream2 = input.parse()?;
    let formula = tokens
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    let invalid = || {
        Error::new_spanned(
            tokens.clone(),
            "invalid `nth_child()` expression; expected an integer, `odd`, `even`, or `An+B`",
        )
    };

    match formula.as_str() {
        "odd" => return Ok((2, 1)),
        "even" => return Ok((2, 0)),
        _ => {}
    }

    if let Some(n_index) = formula.find('n') {
        if formula[n_index + 1..].contains('n') {
            return Err(invalid());
        }

        let coefficient = match &formula[..n_index] {
            "" | "+" => 1,
            "-" => -1,
            value => value.parse::<isize>().map_err(|_| invalid())?,
        };
        let remainder = &formula[n_index + 1..];
        let offset = if remainder.is_empty() {
            0
        } else if let Some(value) = remainder.strip_prefix('+') {
            value.parse::<isize>().map_err(|_| invalid())?
        } else if remainder.starts_with('-') {
            remainder.parse::<isize>().map_err(|_| invalid())?
        } else {
            return Err(invalid());
        };
        Ok((coefficient, offset))
    } else {
        formula
            .parse::<isize>()
            .map(|position| (0, position))
            .map_err(|_| invalid())
    }
}

struct StylePropInput {
    is_custom: bool,
    name: Ident,
    value: StyleValueInput,
}

impl Parse for StylePropInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let is_custom = if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            input.parse::<Token![-]>()?;
            true
        } else {
            false
        };

        let name = input.parse()?;
        input.parse::<Token![:]>()?;

        let value = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            is_custom,
            name,
            value,
        })
    }
}

enum StyleValueInput {
    Literal(String),
    Inserted(Expr),
}

impl Parse for StyleValueInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            let content;
            parenthesized!(content in input);
            return Ok(Self::Inserted(content.parse()?));
        }

        let mut value = TokenStream2::new();
        while !input.peek(Token![;]) {
            if input.is_empty() {
                return Err(input.error("expected `;` after style property value"));
            }

            value.extend([input.parse::<TokenTree>()?]);
        }

        Ok(Self::Literal(css_value_to_string(value)))
    }
}

fn css_value_to_string(value: TokenStream2) -> String {
    let mut output = String::new();
    let mut previous: Option<TokenKind> = None;

    for token in value {
        let current = TokenKind::from(&token);
        let text = match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                format!("({})", css_value_to_string(group.stream()))
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => {
                format!("[{}]", css_value_to_string(group.stream()))
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                format!("{{{}}}", css_value_to_string(group.stream()))
            }
            token => token.to_string(),
        };

        if should_insert_space(previous, current) {
            output.push(' ');
        }

        output.push_str(&text);
        previous = Some(current);
    }

    output
}

#[derive(Copy, Clone)]
enum TokenKind {
    Word,
    Punct,
    Group,
}

impl From<&TokenTree> for TokenKind {
    fn from(token: &TokenTree) -> Self {
        match token {
            TokenTree::Ident(_) | TokenTree::Literal(_) => Self::Word,
            TokenTree::Punct(_) => Self::Punct,
            TokenTree::Group(_) => Self::Group,
        }
    }
}

fn should_insert_space(previous: Option<TokenKind>, current: TokenKind) -> bool {
    matches!(
        (previous, current),
        (Some(TokenKind::Word), TokenKind::Word)
            | (Some(TokenKind::Group), TokenKind::Word)
            | (Some(TokenKind::Word), TokenKind::Group)
    )
}

fn expand_style(input: StyleSheetInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let items = input
        .items
        .into_iter()
        .map(expand_item)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        {
            let mut __nestix_style_sheet = #nestix_native_path::StyleSheet::new(::std::vec![]);
            #(#items)*
            __nestix_style_sheet
        }
    })
}

fn expand_item(item: StyleItemInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();

    match item {
        StyleItemInput::Rule(rule) => {
            let rules = flatten_rule(rule, None)?
                .into_iter()
                .map(expand_rule)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! {
                __nestix_style_sheet.extend(&#nestix_native_path::StyleSheet::new(::std::vec![
                    #(#rules),*
                ]));
            })
        }
        StyleItemInput::Inserted(style_sheet) => Ok(quote! {
            __nestix_style_sheet.extend(&(#style_sheet));
        }),
    }
}

fn flatten_rule(
    rule: StyleRuleInput,
    parent: Option<&SelectorInput>,
) -> Result<Vec<StyleRuleInput>> {
    let selector = resolve_nested_selector(rule.selector, parent)?;
    let mut flattened = vec![StyleRuleInput {
        selector: selector.clone(),
        props: rule.props,
        nested_rules: Vec::new(),
    }];

    for nested_rule in rule.nested_rules {
        flattened.extend(flatten_rule(nested_rule, Some(&selector))?);
    }

    Ok(flattened)
}

fn resolve_nested_selector(
    selector: SelectorInput,
    parent: Option<&SelectorInput>,
) -> Result<SelectorInput> {
    let Some(parent) = parent else {
        if let Some(parent_selector) = selector.parent_selector() {
            return Err(parent_selector.top_level_error());
        }
        return Ok(selector);
    };

    let mut selectors = Vec::new();
    for parent_selector in &parent.selectors {
        for nested_selector in &selector.selectors {
            selectors.push(if nested_selector.contains_parent() {
                nested_selector.replace_parent(parent_selector)
            } else {
                SelectorAst::Descendant(
                    Box::new(parent_selector.clone()),
                    Box::new(nested_selector.clone()),
                )
            });
        }
    }

    Ok(SelectorInput { selectors })
}

impl SelectorInput {
    fn parent_selector(&self) -> Option<ParentSelectorInput> {
        self.selectors.iter().find_map(SelectorAst::parent_selector)
    }
}

impl SelectorAst {
    fn parent_selector(&self) -> Option<ParentSelectorInput> {
        match self {
            Self::Parent(parent) => Some(*parent),
            Self::Not(selector) => selector.parent_selector(),
            Self::All(selectors) => selectors.iter().find_map(Self::parent_selector),
            Self::Child(left, right)
            | Self::Descendant(left, right)
            | Self::AdjacentSibling(left, right)
            | Self::SubsequentSibling(left, right) => {
                left.parent_selector().or_else(|| right.parent_selector())
            }
            Self::Class(_) | Self::FirstChild | Self::LastChild | Self::NthChild { .. } => None,
        }
    }

    fn contains_parent(&self) -> bool {
        self.parent_selector().is_some()
    }

    fn replace_parent(&self, parent: &Self) -> Self {
        match self {
            Self::Parent(_) => parent.clone(),
            Self::Class(class) => Self::Class(class.clone()),
            Self::Not(selector) => Self::Not(SelectorInput {
                selectors: selector
                    .selectors
                    .iter()
                    .map(|selector| selector.replace_parent(parent))
                    .collect(),
            }),
            Self::FirstChild => Self::FirstChild,
            Self::LastChild => Self::LastChild,
            Self::NthChild { a, b } => Self::NthChild { a: *a, b: *b },
            Self::All(selectors) => Self::All(
                selectors
                    .iter()
                    .map(|selector| selector.replace_parent(parent))
                    .collect(),
            ),
            Self::Child(left, right) => Self::Child(
                Box::new(left.replace_parent(parent)),
                Box::new(right.replace_parent(parent)),
            ),
            Self::Descendant(left, right) => Self::Descendant(
                Box::new(left.replace_parent(parent)),
                Box::new(right.replace_parent(parent)),
            ),
            Self::AdjacentSibling(left, right) => Self::AdjacentSibling(
                Box::new(left.replace_parent(parent)),
                Box::new(right.replace_parent(parent)),
            ),
            Self::SubsequentSibling(left, right) => Self::SubsequentSibling(
                Box::new(left.replace_parent(parent)),
                Box::new(right.replace_parent(parent)),
            ),
        }
    }
}

impl ParentSelectorInput {
    fn top_level_error(self) -> Error {
        match self {
            Self::Explicit(span) => Error::new(
                span,
                "the `&` parent selector is only allowed inside a nested style rule",
            ),
            Self::Relative(span) => Error::new(
                span,
                "a leading selector combinator is only allowed inside a nested style rule",
            ),
        }
    }
}

fn expand_rule(rule: StyleRuleInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let selector = expand_selector(rule.selector);
    let declarations = rule
        .props
        .into_iter()
        .map(expand_declaration)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #nestix_native_path::StyleRule {
            selector: #selector,
            declarations: ::std::vec![
                #(#declarations),*
            ],
        }
    })
}

fn expand_declaration(prop: StylePropInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let name = prop.name.to_string();
    let name_span = prop.name.span();
    let value = prop.value;

    if prop.is_custom {
        let name = format!("--{}", name);
        let value = match value {
            StyleValueInput::Literal(value) => quote!(#value.to_string()),
            StyleValueInput::Inserted(value) => quote!((#value).to_string()),
        };
        return Ok(quote! {
            #nestix_native_path::StyleDeclaration::Custom {
                name: #name.to_string(),
                value: #value,
            }
        });
    }

    if let StyleValueInput::Literal(global) = &value
        && matches!(global.as_str(), "inherit" | "initial" | "unset")
    {
        let property = expand_property_variant(&name, name_span)?;
        let global = Ident::new(
            match global.as_str() {
                "inherit" => "Inherit",
                "initial" => "Initial",
                "unset" => "Unset",
                _ => unreachable!(),
            },
            Span::call_site(),
        );
        return Ok(quote! {
            #nestix_native_path::StyleDeclaration::Property(
                #nestix_native_path::StyleProperty::#property(
                    #nestix_native_path::StyleValue::#global
                )
            )
        });
    }

    let value = match name.as_str() {
        "appearance" => expand_appearance(value)?,
        "bg_color" => expand_color(value)?,
        "font_family" => expand_font_family(value)?,
        "font_size" => expand_font_size(value)?,
        "font_weight" => expand_font_weight(value)?,
        "font_style" => expand_font_style(value)?,
        "text_color" => expand_color(value)?,
        "left" => expand_dimension(value)?,
        "top" => expand_dimension(value)?,
        "width" => expand_dimension(value)?,
        "height" => expand_dimension(value)?,
        "margin" => expand_dimension(value)?,
        "margin_horizontal" => expand_dimension(value)?,
        "margin_vertical" => expand_dimension(value)?,
        "margin_left" => expand_dimension(value)?,
        "margin_right" => expand_dimension(value)?,
        "margin_top" => expand_dimension(value)?,
        "margin_bottom" => expand_dimension(value)?,
        "padding" => expand_dimension(value)?,
        "padding_horizontal" => expand_dimension(value)?,
        "padding_vertical" => expand_dimension(value)?,
        "padding_left" => expand_dimension(value)?,
        "padding_right" => expand_dimension(value)?,
        "padding_top" => expand_dimension(value)?,
        "padding_bottom" => expand_dimension(value)?,
        "flex_grow" => expand_f32(value)?,
        "flex_basis" => expand_dimension(value)?,
        "flex_shrink" => expand_f32(value)?,
        "align_self" => expand_align_items(value)?,
        "flex_direction" => expand_flex_direction(value)?,
        "align_items" => expand_align_items(value)?,
        "justify_content" => expand_justify_content(value)?,
        "flex_wrap" => expand_flex_wrap(value)?,
        "gap" => expand_dimension(value)?,
        _ => Err(Error::new_spanned(
            prop.name,
            format!(
                "unknown built-in style property `{}`; use a `--` prefix for custom properties",
                name
            ),
        ))?,
    };

    Ok(expand_property(
        expand_property_variant(&name, name_span)?,
        value,
    ))
}

fn expand_property_variant(name: &str, span: Span) -> Result<Ident> {
    let variant = match name {
        "appearance" => "Appearance",
        "bg_color" => "BgColor",
        "font_family" => "FontFamily",
        "font_size" => "FontSize",
        "font_weight" => "FontWeight",
        "font_style" => "FontStyle",
        "text_color" => "TextColor",
        "left" => "Left",
        "top" => "Top",
        "width" => "Width",
        "height" => "Height",
        "margin" => "Margin",
        "margin_horizontal" => "MarginHorizontal",
        "margin_vertical" => "MarginVertical",
        "margin_left" => "MarginLeft",
        "margin_right" => "MarginRight",
        "margin_top" => "MarginTop",
        "margin_bottom" => "MarginBottom",
        "padding" => "Padding",
        "padding_horizontal" => "PaddingHorizontal",
        "padding_vertical" => "PaddingVertical",
        "padding_left" => "PaddingLeft",
        "padding_right" => "PaddingRight",
        "padding_top" => "PaddingTop",
        "padding_bottom" => "PaddingBottom",
        "flex_grow" => "FlexGrow",
        "flex_basis" => "FlexBasis",
        "flex_shrink" => "FlexShrink",
        "align_self" => "AlignSelf",
        "flex_direction" => "FlexDirection",
        "align_items" => "AlignItems",
        "justify_content" => "JustifyContent",
        "flex_wrap" => "FlexWrap",
        "gap" => "Gap",
        _ => {
            return Err(Error::new(
                span,
                format!(
                    "unknown built-in style property `{}`; use a `--` prefix for custom properties",
                    name
                ),
            ));
        }
    };
    Ok(Ident::new(variant, span))
}

fn expand_property(variant: Ident, value: TokenStream2) -> TokenStream2 {
    let nestix_native_path = nestix_native_path();
    quote! {
        #nestix_native_path::StyleDeclaration::Property(
            #nestix_native_path::StyleProperty::#variant(
                #nestix_native_path::StyleValue::Value(#value)
            )
        )
    }
}

fn expand_appearance(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };
    let variant = match value.as_str() {
        "native" => "Native",
        "none" => "None",
        "auto" => "Auto",
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "appearance must be native, none, auto, or an inserted Appearance",
            ));
        }
    };
    let variant = Ident::new(variant, Span::call_site());
    Ok(quote!(#nestix_native_path::Appearance::#variant))
}

fn expand_color(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "white" => return Ok(quote!(#nestix_native_path::Color::WHITE)),
        "black" => return Ok(quote!(#nestix_native_path::Color::BLACK)),
        "transparent" => return Ok(quote!(#nestix_native_path::Color::TRANSPARENT)),
        "red" => return Ok(quote!(#nestix_native_path::Color::RED)),
        "green" => return Ok(quote!(#nestix_native_path::Color::GREEN)),
        "blue" => return Ok(quote!(#nestix_native_path::Color::BLUE)),
        _ => {}
    }

    let hex = value.strip_prefix('#').unwrap_or(&value);
    if hex.len() != 6 && hex.len() != 8 {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "color values must be named colors or 6/8 digit hex colors",
        ));
    }

    let red = parse_hex_pair(hex, 0)?;
    let green = parse_hex_pair(hex, 2)?;
    let blue = parse_hex_pair(hex, 4)?;
    let alpha = if hex.len() == 8 {
        parse_hex_pair(hex, 6)?
    } else {
        255
    };

    Ok(quote! {
        #nestix_native_path::Color::RGB(#nestix_native_path::RGBColor::from_rgba(#red, #green, #blue, #alpha))
    })
}

fn expand_dimension(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    if value == "auto" {
        return Ok(quote!(#nestix_native_path::Dimension::Auto));
    }

    let Some(value) = value.strip_suffix("px") else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "dimension values must be `auto`, `{number}px`, or an inserted Dimension",
        ));
    };

    let dimension = value.parse::<f64>().map_err(|_| {
        Error::new(
            proc_macro2::Span::call_site(),
            "dimension values must be `auto`, `{number}px`, or an inserted Dimension",
        )
    })?;

    Ok(quote!(#nestix_native_path::Dimension::from(#dimension)))
}

fn expand_f32(value: StyleValueInput) -> Result<TokenStream2> {
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    let value = value.parse::<f32>().map_err(|_| {
        Error::new(
            proc_macro2::Span::call_site(),
            "f32 style values must be numbers or inserted f32 values",
        )
    })?;

    Ok(quote!(#value))
}

fn expand_font_family(value: StyleValueInput) -> Result<TokenStream2> {
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };
    let value = match syn::parse_str::<syn::LitStr>(&value) {
        Ok(literal) => literal.value(),
        Err(_) if value.split_whitespace().count() == 1 => value,
        Err(_) => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "font-family names containing spaces must be wrapped in double quotes",
            ));
        }
    };
    if value.trim().is_empty() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "font-family must not be empty",
        ));
    }
    Ok(quote!(#value.to_string()))
}

fn expand_font_size(value: StyleValueInput) -> Result<TokenStream2> {
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };
    let Some(value) = value.strip_suffix("px") else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "font-size must be `{number}px` or an inserted f64",
        ));
    };
    let value = value.parse::<f64>().map_err(|_| {
        Error::new(
            proc_macro2::Span::call_site(),
            "font-size must be `{number}px` or an inserted f64",
        )
    })?;
    if !value.is_finite() || value <= 0.0 {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "font-size must be greater than zero",
        ));
    }
    Ok(quote!(#value))
}

fn expand_font_weight(value: StyleValueInput) -> Result<TokenStream2> {
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };
    if let Some(weight) = parse_numeric_font_weight(&value)? {
        let nestix_native_path = nestix_native_path();
        return Ok(quote!(#nestix_native_path::FontWeight::Numeric(#weight)));
    }
    let nestix_native_path = nestix_native_path();
    let variant = match value.as_str() {
        "thin" => "Thin",
        "extra_light" | "extra-light" => "ExtraLight",
        "light" => "Light",
        "normal" => "Normal",
        "medium" => "Medium",
        "semi_bold" | "semi-bold" | "semibold" => "SemiBold",
        "bold" => "Bold",
        "extra_bold" | "extra-bold" => "ExtraBold",
        "black" => "Black",
        _ => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "font-weight must be a number from 1 to 1000, thin, extra-light, light, normal, medium, semi-bold, bold, extra-bold, black, or an inserted FontWeight",
            ));
        }
    };
    let variant = Ident::new(variant, Span::call_site());
    Ok(quote!(#nestix_native_path::FontWeight::#variant))
}

fn parse_numeric_font_weight(value: &str) -> Result<Option<u16>> {
    let Ok(weight) = value.parse::<u16>() else {
        return Ok(None);
    };
    if (1..=1000).contains(&weight) {
        Ok(Some(weight))
    } else {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            "numeric font-weight must be between 1 and 1000 inclusive",
        ))
    }
}

fn expand_font_style(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };
    match value.as_str() {
        "normal" => Ok(quote!(#nestix_native_path::FontStyle::Normal)),
        "italic" => Ok(quote!(#nestix_native_path::FontStyle::Italic)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "font-style must be normal, italic, or an inserted FontStyle",
        )),
    }
}

fn expand_align_items(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "normal" => Ok(quote!(#nestix_native_path::AlignItems::Normal)),
        "start" => Ok(quote!(#nestix_native_path::AlignItems::Start)),
        "end" => Ok(quote!(#nestix_native_path::AlignItems::End)),
        "flex_start" | "flex-start" => Ok(quote!(#nestix_native_path::AlignItems::FlexStart)),
        "flex_end" | "flex-end" => Ok(quote!(#nestix_native_path::AlignItems::FlexEnd)),
        "center" => Ok(quote!(#nestix_native_path::AlignItems::Center)),
        "baseline" => Ok(quote!(#nestix_native_path::AlignItems::Baseline)),
        "stretch" => Ok(quote!(#nestix_native_path::AlignItems::Stretch)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "align-self must be normal, start, end, flex-start, flex-end, center, baseline, stretch, or an inserted AlignItems",
        )),
    }
}

fn expand_flex_direction(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "row" => Ok(quote!(#nestix_native_path::FlexDirection::Row)),
        "row_reverse" | "row-reverse" => Ok(quote!(#nestix_native_path::FlexDirection::RowReverse)),
        "column" => Ok(quote!(#nestix_native_path::FlexDirection::Column)),
        "column_reverse" | "column-reverse" => {
            Ok(quote!(#nestix_native_path::FlexDirection::ColumnReverse))
        }
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "flex-direction must be row, row-reverse, column, column-reverse, or an inserted FlexDirection",
        )),
    }
}

fn expand_justify_content(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "normal" => Ok(quote!(#nestix_native_path::JustifyContent::Normal)),
        "start" => Ok(quote!(#nestix_native_path::JustifyContent::Start)),
        "end" => Ok(quote!(#nestix_native_path::JustifyContent::End)),
        "flex_start" | "flex-start" => Ok(quote!(#nestix_native_path::JustifyContent::FlexStart)),
        "flex_end" | "flex-end" => Ok(quote!(#nestix_native_path::JustifyContent::FlexEnd)),
        "center" => Ok(quote!(#nestix_native_path::JustifyContent::Center)),
        "stretch" => Ok(quote!(#nestix_native_path::JustifyContent::Stretch)),
        "space_between" | "space-between" => {
            Ok(quote!(#nestix_native_path::JustifyContent::SpaceBetween))
        }
        "space_evenly" | "space-evenly" => {
            Ok(quote!(#nestix_native_path::JustifyContent::SpaceEvenly))
        }
        "space_around" | "space-around" => {
            Ok(quote!(#nestix_native_path::JustifyContent::SpaceAround))
        }
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "justify-content must be normal, start, end, flex-start, flex-end, center, stretch, space-between, space-evenly, space-around, or an inserted JustifyContent",
        )),
    }
}

fn expand_flex_wrap(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "nowrap" | "no_wrap" | "no-wrap" => Ok(quote!(#nestix_native_path::FlexWrap::NoWrap)),
        "wrap" => Ok(quote!(#nestix_native_path::FlexWrap::Wrap)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "flex-wrap must be wrap, no-wrap, nowrap, or an inserted FlexWrap",
        )),
    }
}

fn parse_hex_pair(hex: &str, index: usize) -> Result<u8> {
    u8::from_str_radix(&hex[index..index + 2], 16).map_err(|_| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("invalid hex color component `{}`", &hex[index..index + 2]),
        )
    })
}

fn expand_selector(selector: SelectorInput) -> TokenStream2 {
    let nestix_native_path = nestix_native_path();
    let selectors = selector
        .selectors
        .into_iter()
        .map(expand_selector_ast)
        .collect::<Vec<_>>();

    if selectors.len() == 1 {
        selectors.into_iter().next().unwrap()
    } else {
        quote! {
            #nestix_native_path::StyleSelector::List(::std::vec![
                #(#selectors),*
            ])
        }
    }
}

fn expand_selector_ast(selector: SelectorAst) -> TokenStream2 {
    let nestix_native_path = nestix_native_path();
    match selector {
        SelectorAst::Class(class) => {
            quote! {
                #nestix_native_path::StyleSelector::Class(#class.to_string())
            }
        }
        SelectorAst::Not(selector) => {
            let selector = expand_selector(selector);
            quote! {
                #nestix_native_path::StyleSelector::Not(::std::boxed::Box::new(#selector))
            }
        }
        SelectorAst::FirstChild => {
            quote! { #nestix_native_path::StyleSelector::FirstChild }
        }
        SelectorAst::LastChild => {
            quote! { #nestix_native_path::StyleSelector::LastChild }
        }
        SelectorAst::NthChild { a, b } => {
            quote! { #nestix_native_path::StyleSelector::NthChild { a: #a, b: #b } }
        }
        SelectorAst::All(selectors) => {
            let selectors = selectors.into_iter().map(expand_selector_ast);
            quote! {
                #nestix_native_path::StyleSelector::All(::std::vec![
                    #(#selectors),*
                ])
            }
        }
        SelectorAst::Child(parent, child) => {
            let parent = expand_selector_ast(*parent);
            let child = expand_selector_ast(*child);
            quote! {
                #nestix_native_path::StyleSelector::Child {
                    parent: ::std::boxed::Box::new(#parent),
                    child: ::std::boxed::Box::new(#child),
                }
            }
        }
        SelectorAst::Descendant(ancestor, descendant) => {
            let ancestor = expand_selector_ast(*ancestor);
            let descendant = expand_selector_ast(*descendant);
            quote! {
                #nestix_native_path::StyleSelector::Descendant {
                    ancestor: ::std::boxed::Box::new(#ancestor),
                    descendant: ::std::boxed::Box::new(#descendant),
                }
            }
        }
        SelectorAst::AdjacentSibling(previous, sibling) => {
            let previous = expand_selector_ast(*previous);
            let sibling = expand_selector_ast(*sibling);
            quote! {
                #nestix_native_path::StyleSelector::AdjacentSibling {
                    previous: ::std::boxed::Box::new(#previous),
                    sibling: ::std::boxed::Box::new(#sibling),
                }
            }
        }
        SelectorAst::SubsequentSibling(previous, sibling) => {
            let previous = expand_selector_ast(*previous);
            let sibling = expand_selector_ast(*sibling);
            quote! {
                #nestix_native_path::StyleSelector::SubsequentSibling {
                    previous: ::std::boxed::Box::new(#previous),
                    sibling: ::std::boxed::Box::new(#sibling),
                }
            }
        }
        SelectorAst::Parent(_) => {
            unreachable!("parent selectors must be resolved before macro expansion")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SelectorAst, SelectorInput, StyleItemInput, StyleSheetInput, StyleValueInput,
        expand_font_family, flatten_rule, parse_numeric_font_weight,
    };

    fn parse_rule(source: &str) -> super::StyleRuleInput {
        let mut input = syn::parse_str::<StyleSheetInput>(source).unwrap();
        assert_eq!(input.items.len(), 1);
        match input.items.pop().unwrap() {
            StyleItemInput::Rule(rule) => rule,
            StyleItemInput::Inserted(_) => panic!("expected a style rule"),
        }
    }

    #[test]
    fn font_family_with_spaces_requires_double_quotes() {
        let error =
            expand_font_family(StyleValueInput::Literal("Comic Sans MS".to_string())).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("must be wrapped in double quotes")
        );
    }

    #[test]
    fn quoted_font_family_with_spaces_is_allowed() {
        assert!(
            expand_font_family(StyleValueInput::Literal("\"Comic Sans MS\"".to_string())).is_ok()
        );
    }

    #[test]
    fn numeric_font_weight_accepts_full_supported_range() {
        assert_eq!(parse_numeric_font_weight("1").unwrap(), Some(1));
        assert_eq!(parse_numeric_font_weight("400").unwrap(), Some(400));
        assert_eq!(parse_numeric_font_weight("1000").unwrap(), Some(1000));
    }

    #[test]
    fn numeric_font_weight_rejects_values_outside_supported_range() {
        for value in ["0", "1001"] {
            let error = parse_numeric_font_weight(value).unwrap_err();
            assert!(error.to_string().contains("between 1 and 1000"));
        }
    }

    #[test]
    fn nth_child_formulas_are_normalized() {
        for (selector, expected) in [
            (":nth_child(odd)", (2, 1)),
            (":nth_child(even)", (2, 0)),
            (":nth_child(4)", (0, 4)),
            (":nth_child(n)", (1, 0)),
            (":nth_child(2n + 1)", (2, 1)),
            (":nth_child(-n + 3)", (-1, 3)),
        ] {
            let parsed = syn::parse_str::<SelectorInput>(selector).unwrap();
            assert!(matches!(
                parsed.selectors.as_slice(),
                [SelectorAst::NthChild { a, b }] if (*a, *b) == expected
            ));
        }
    }

    #[test]
    fn nth_child_rejects_invalid_formulas() {
        for selector in [":nth_child()", ":nth_child(2n+)", ":nth_child(nonsense)"] {
            let error = syn::parse_str::<SelectorInput>(selector).err().unwrap();
            assert!(
                error.to_string().contains("nth_child"),
                "unexpected error for {selector}: {error}"
            );
        }
    }

    #[test]
    fn nested_selector_lists_expand_as_a_cartesian_product() {
        let rules = flatten_rule(
            parse_rule(".panel, .dialog { .title, .subtitle { bg_color: blue; } }"),
            None,
        )
        .unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector.selectors.len(), 2);
        assert_eq!(rules[1].selector.selectors.len(), 4);
        assert!(
            rules[1]
                .selector
                .selectors
                .iter()
                .all(|selector| matches!(selector, SelectorAst::Descendant(_, _)))
        );
    }

    #[test]
    fn nested_parent_references_and_relative_combinators_are_resolved() {
        let rules = flatten_rule(
            parse_rule(".panel { &.selected { > .button { + .icon { bg_color: blue; } } } }"),
            None,
        )
        .unwrap();

        assert_eq!(rules.len(), 4);
        assert!(matches!(
            &rules[1].selector.selectors[0],
            SelectorAst::All(_)
        ));
        assert!(matches!(
            &rules[2].selector.selectors[0],
            SelectorAst::Child(_, _)
        ));
        assert!(matches!(
            &rules[3].selector.selectors[0],
            SelectorAst::AdjacentSibling(_, _)
        ));
        assert!(
            rules
                .iter()
                .all(|rule| rule.selector.parent_selector().is_none())
        );
    }

    #[test]
    fn top_level_parent_references_and_relative_combinators_are_rejected() {
        for (source, expected) in [
            ("&.selected { bg_color: blue; }", "parent selector"),
            (
                "> .button { bg_color: blue; }",
                "leading selector combinator",
            ),
        ] {
            let error = flatten_rule(parse_rule(source), None).err().unwrap();
            assert!(
                error.to_string().contains(expected),
                "unexpected error for {source}: {error}"
            );
        }
    }

    #[test]
    fn parent_selector_interpolation_is_rejected() {
        let error = syn::parse_str::<StyleSheetInput>(".panel { &-selected { bg_color: blue; } }")
            .err()
            .unwrap();
        assert!(error.to_string().contains("interpolation"));
    }
}
