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
}

impl Parse for StyleRuleInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;

        let content;
        braced!(content in input);

        let mut props = Vec::new();
        while !content.is_empty() {
            props.push(content.parse()?);
        }

        Ok(Self { selector, props })
    }
}

struct SelectorInput {
    selectors: Vec<SelectorAst>,
}

enum SelectorAst {
    Class(String),
    Not(SelectorInput),
    All(Vec<SelectorAst>),
    Child(Box<SelectorAst>, Box<SelectorAst>),
    Descendant(Box<SelectorAst>, Box<SelectorAst>),
    AdjacentSibling(Box<SelectorAst>, Box<SelectorAst>),
    SubsequentSibling(Box<SelectorAst>, Box<SelectorAst>),
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
    let mut selector = parse_compound_selector(input)?;

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

    if name != "not" {
        return Err(Error::new_spanned(
            name,
            "unsupported pseudo selector; expected `not`",
        ));
    }

    let content;
    parenthesized!(content in input);
    Ok(SelectorAst::Not(content.parse()?))
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
            let rule = expand_rule(rule)?;
            Ok(quote! {
                __nestix_style_sheet.extend(&#nestix_native_path::StyleSheet::new(::std::vec![
                    #rule
                ]));
            })
        }
        StyleItemInput::Inserted(style_sheet) => Ok(quote! {
            __nestix_style_sheet.extend(&(#style_sheet));
        }),
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

    let (variant, value) = match name.as_str() {
        "bg_color" => ("BgColor", expand_color(value)?),
        "left" => ("Left", expand_dimension(value)?),
        "top" => ("Top", expand_dimension(value)?),
        "width" => ("Width", expand_dimension(value)?),
        "height" => ("Height", expand_dimension(value)?),
        "margin" => ("Margin", expand_dimension(value)?),
        "margin_horizontal" => ("MarginHorizontal", expand_dimension(value)?),
        "margin_vertical" => ("MarginVertical", expand_dimension(value)?),
        "margin_left" => ("MarginLeft", expand_dimension(value)?),
        "margin_right" => ("MarginRight", expand_dimension(value)?),
        "margin_top" => ("MarginTop", expand_dimension(value)?),
        "margin_bottom" => ("MarginBottom", expand_dimension(value)?),
        "padding" => ("Padding", expand_dimension(value)?),
        "padding_horizontal" => ("PaddingHorizontal", expand_dimension(value)?),
        "padding_vertical" => ("PaddingVertical", expand_dimension(value)?),
        "padding_left" => ("PaddingLeft", expand_dimension(value)?),
        "padding_right" => ("PaddingRight", expand_dimension(value)?),
        "padding_top" => ("PaddingTop", expand_dimension(value)?),
        "padding_bottom" => ("PaddingBottom", expand_dimension(value)?),
        "grow" => ("Grow", expand_f32(value)?),
        "align_self" => ("AlignSelf", expand_align_items(value)?),
        "flex_direction" => ("FlexDirection", expand_flex_direction(value)?),
        "align_items" => ("AlignItems", expand_align_items(value)?),
        "justify_content" => ("JustifyContent", expand_justify_content(value)?),
        "flex_wrap" => ("FlexWrap", expand_flex_wrap(value)?),
        "gap" => ("Gap", expand_dimension(value)?),
        _ => Err(Error::new_spanned(
            prop.name,
            format!(
                "unknown built-in style property `{}`; use a `--` prefix for custom properties",
                name
            ),
        ))?,
    };

    Ok(expand_property(variant, value, name_span))
}

fn expand_property(variant: &str, value: TokenStream2, span: Span) -> TokenStream2 {
    let nestix_native_path = nestix_native_path();
    let variant = Ident::new(variant, span);
    quote! {
        #nestix_native_path::StyleDeclaration::Property(
            #nestix_native_path::StyleProperty::#variant(#value)
        )
    }
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

fn expand_align_items(value: StyleValueInput) -> Result<TokenStream2> {
    let nestix_native_path = nestix_native_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "unset" => Ok(quote!(#nestix_native_path::AlignItems::Unset)),
        "start" => Ok(quote!(#nestix_native_path::AlignItems::Start)),
        "end" => Ok(quote!(#nestix_native_path::AlignItems::End)),
        "flex_start" | "flex-start" => Ok(quote!(#nestix_native_path::AlignItems::FlexStart)),
        "flex_end" | "flex-end" => Ok(quote!(#nestix_native_path::AlignItems::FlexEnd)),
        "center" => Ok(quote!(#nestix_native_path::AlignItems::Center)),
        "baseline" => Ok(quote!(#nestix_native_path::AlignItems::Baseline)),
        "stretch" => Ok(quote!(#nestix_native_path::AlignItems::Stretch)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "align-self must be unset, start, end, flex-start, flex-end, center, baseline, stretch, or an inserted AlignItems",
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
        "unset" => Ok(quote!(#nestix_native_path::JustifyContent::Unset)),
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
            "justify-content must be unset, start, end, flex-start, flex-end, center, stretch, space-between, space-evenly, space-around, or an inserted JustifyContent",
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
    }
}
