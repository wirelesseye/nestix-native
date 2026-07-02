use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{
    Error, Expr, Ident, Result, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

use crate::util::core_path;

pub fn style(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as StyleSheetInput);
    expand_style(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct StyleSheetInput {
    rules: Vec<StyleRuleInput>,
}

impl Parse for StyleSheetInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }

        Ok(Self { rules })
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
    class_lists: Vec<Vec<String>>,
}

impl Parse for SelectorInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut class_lists = Vec::new();

        loop {
            class_lists.push(parse_class_list(input)?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self { class_lists })
    }
}

fn parse_class_list(input: ParseStream<'_>) -> Result<Vec<String>> {
    let mut classes = Vec::new();

    loop {
        input.parse::<Token![.]>()?;
        let class_name: Ident = input.parse()?;
        classes.push(class_name.to_string());

        if !input.peek(Token![.]) {
            break;
        }
    }

    Ok(classes)
}

struct StylePropInput {
    name: String,
    value: StyleValueInput,
}

impl Parse for StylePropInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = parse_prop_name(input)?;
        input.parse::<Token![:]>()?;

        let value = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self { name, value })
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

fn parse_prop_name(input: ParseStream<'_>) -> Result<String> {
    if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        input.parse::<Token![-]>()?;
        let mut name = "--".to_string();
        name.push_str(&parse_prop_name_segment(input)?);

        while input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            name.push('-');
            name.push_str(&parse_prop_name_segment(input)?);
        }

        return Ok(name);
    }

    let mut name = input.parse::<Ident>()?.to_string();

    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        name.push('-');
        name.push_str(&parse_prop_name_segment(input)?);
    }

    Ok(name)
}

fn parse_prop_name_segment(input: ParseStream<'_>) -> Result<String> {
    match input.parse::<TokenTree>()? {
        TokenTree::Ident(ident) => Ok(ident.to_string()),
        token => Err(Error::new_spanned(
            token,
            "expected style property name segment",
        )),
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
    let core_path = core_path();
    let rules = input
        .rules
        .into_iter()
        .map(expand_rule)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #core_path::StyleSheet::new(::std::vec![
            #(#rules),*
        ])
    })
}

fn expand_rule(rule: StyleRuleInput) -> Result<TokenStream2> {
    let core_path = core_path();
    let selector = expand_selector(rule.selector);
    let declarations = rule
        .props
        .into_iter()
        .map(expand_declaration)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    Ok(quote! {
        #core_path::StyleRule {
            selector: #selector,
            declarations: ::std::vec![
                #(#declarations),*
            ],
        }
    })
}

fn expand_declaration(prop: StylePropInput) -> Result<Vec<TokenStream2>> {
    let core_path = core_path();
    let name = canonical_prop_name(&prop.name);
    let value = prop.value;

    if name.starts_with("--") {
        let value = match value {
            StyleValueInput::Literal(value) => quote!(#value.to_string()),
            StyleValueInput::Inserted(value) => quote!((#value).to_string()),
        };
        return Ok(vec![quote! {
            #core_path::StyleDeclaration::Custom {
                name: #name.to_string(),
                value: #value,
            }
        }]);
    }

    match name.as_str() {
        "bg_color" => {
            let color = expand_color(value)?;
            Ok(vec![expand_property("BgColor", color)])
        }
        "left" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("Left", dimension)])
        }
        "top" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("Top", dimension)])
        }
        "width" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("Width", dimension)])
        }
        "height" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("Height", dimension)])
        }
        "margin" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![
                expand_property("MarginTop", dimension.clone()),
                expand_property("MarginRight", dimension.clone()),
                expand_property("MarginBottom", dimension.clone()),
                expand_property("MarginLeft", dimension),
            ])
        }
        "margin_horizontal" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![
                expand_property("MarginLeft", dimension.clone()),
                expand_property("MarginRight", dimension),
            ])
        }
        "margin_vertical" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![
                expand_property("MarginTop", dimension.clone()),
                expand_property("MarginBottom", dimension),
            ])
        }
        "margin_left" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("MarginLeft", dimension)])
        }
        "margin_right" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("MarginRight", dimension)])
        }
        "margin_top" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("MarginTop", dimension)])
        }
        "margin_bottom" => {
            let dimension = expand_dimension(value)?;
            Ok(vec![expand_property("MarginBottom", dimension)])
        }
        "grow" => {
            let grow = expand_f32(value)?;
            Ok(vec![expand_property("Grow", grow)])
        }
        "align_self" => {
            let align_self = expand_align_items(value)?;
            Ok(vec![expand_property("AlignSelf", align_self)])
        }
        "flex_direction" => {
            let flex_direction = expand_flex_direction(value)?;
            Ok(vec![expand_property("FlexDirection", flex_direction)])
        }
        "align_items" => {
            let align_items = expand_align_items(value)?;
            Ok(vec![expand_property("AlignItems", align_items)])
        }
        "flex_wrap" => {
            let flex_wrap = expand_flex_wrap(value)?;
            Ok(vec![expand_property("FlexWrap", flex_wrap)])
        }
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "unknown built-in style property `{}`; use a `--` prefix for custom properties",
                prop.name
            ),
        )),
    }
}

fn expand_property(variant: &str, value: TokenStream2) -> TokenStream2 {
    let core_path = core_path();
    let variant = Ident::new(variant, proc_macro2::Span::call_site());
    quote! {
        #core_path::StyleDeclaration::Property(
            #core_path::StyleProperty::#variant(#value)
        )
    }
}

fn canonical_prop_name(name: &str) -> String {
    if name.starts_with("--") {
        name.to_string()
    } else {
        name.replace('-', "_")
    }
}

fn expand_color(value: StyleValueInput) -> Result<TokenStream2> {
    let core_path = core_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "white" => return Ok(quote!(#core_path::Color::WHITE)),
        "black" => return Ok(quote!(#core_path::Color::BLACK)),
        "transparent" => return Ok(quote!(#core_path::Color::TRANSPARENT)),
        "red" => return Ok(quote!(#core_path::Color::RED)),
        "green" => return Ok(quote!(#core_path::Color::GREEN)),
        "blue" => return Ok(quote!(#core_path::Color::BLUE)),
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
        #core_path::Color::RGB(#core_path::RGBColor::from_rgba(#red, #green, #blue, #alpha))
    })
}

fn expand_dimension(value: StyleValueInput) -> Result<TokenStream2> {
    let core_path = core_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    if value == "auto" {
        return Ok(quote!(#core_path::Dimension::Auto));
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

    Ok(quote!(#core_path::Dimension::from(#dimension)))
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
    let core_path = core_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "unset" => Ok(quote!(#core_path::AlignItems::Unset)),
        "start" => Ok(quote!(#core_path::AlignItems::Start)),
        "end" => Ok(quote!(#core_path::AlignItems::End)),
        "flex_start" | "flex-start" => Ok(quote!(#core_path::AlignItems::FlexStart)),
        "flex_end" | "flex-end" => Ok(quote!(#core_path::AlignItems::FlexEnd)),
        "center" => Ok(quote!(#core_path::AlignItems::Center)),
        "baseline" => Ok(quote!(#core_path::AlignItems::Baseline)),
        "stretch" => Ok(quote!(#core_path::AlignItems::Stretch)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "align-self must be unset, start, end, flex-start, flex-end, center, baseline, stretch, or an inserted AlignItems",
        )),
    }
}

fn expand_flex_direction(value: StyleValueInput) -> Result<TokenStream2> {
    let core_path = core_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "row" => Ok(quote!(#core_path::FlexDirection::Row)),
        "row_reverse" | "row-reverse" => Ok(quote!(#core_path::FlexDirection::RowReverse)),
        "column" => Ok(quote!(#core_path::FlexDirection::Column)),
        "column_reverse" | "column-reverse" => Ok(quote!(#core_path::FlexDirection::ColumnReverse)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "flex-direction must be row, row-reverse, column, column-reverse, or an inserted FlexDirection",
        )),
    }
}

fn expand_flex_wrap(value: StyleValueInput) -> Result<TokenStream2> {
    let core_path = core_path();
    let value = match value {
        StyleValueInput::Inserted(value) => return Ok(quote!(#value)),
        StyleValueInput::Literal(value) => value,
    };

    match value.as_str() {
        "nowrap" | "no_wrap" | "no-wrap" => Ok(quote!(#core_path::FlexWrap::NoWrap)),
        "wrap" => Ok(quote!(#core_path::FlexWrap::Wrap)),
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
    let core_path = core_path();
    let selectors = selector
        .class_lists
        .into_iter()
        .map(|classes| {
            let class_list = classes.join(" ");
            quote! {
                #core_path::StyleSelector::Class(#class_list.into())
            }
        })
        .collect::<Vec<_>>();

    if selectors.len() == 1 {
        selectors.into_iter().next().unwrap()
    } else {
        quote! {
            #core_path::StyleSelector::List(::std::vec![
                #(#selectors),*
            ])
        }
    }
}
