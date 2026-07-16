mod style;
mod utils;

use proc_macro::TokenStream;

/// Builds a Nestix Native stylesheet.
///
/// The macro accepts class selectors and typed built-in style properties. A
/// stylesheet with only literal values expands to a
/// `nestix_native_core::StyleSheet`.
///
/// ```rust,ignore
/// # use nestix_native_core::*;
/// let styles = style! {
///     .counter, .__Button {
///         bg_color: #FFFFFF;
///         width: 120px;
///         margin: 8px;
///         margin_left: 16px;
///         flex_grow: 1;
///         align_self: center;
///         font_family: "Helvetica Neue";
///         font_size: 14px;
///         font_weight: semi-bold;
///         font_style: italic;
///         text_color: black;
///         --accent_token: primary;
///     }
/// };
/// ```
///
/// Selectors support `:not()`, `:first_child`, `:last_child`, and CSS-style
/// `:nth_child(An+B)` expressions such as `2`, `odd`, `even`, `2n+1`, and
/// `-n+3`. Child positions are one-based and are evaluated among logical
/// style-participating siblings; transparent component wrappers do not add a
/// position.
///
/// Dimension literals must be `auto` or a pixel value such as `30px`.
/// Bare numeric dimensions such as `margin: 30;` are rejected. Built-in values
/// are parsed as their Rust types, while custom properties must use a `--`
/// prefix and are stored as strings.
/// Font family, size, weight, style, and text color are inherited by nested
/// style scopes unless overridden by the child. Font family names containing
/// spaces must be wrapped in double quotes, such as `"Comic Sans MS"`.
/// Font weights may also be numeric values from `1` through `1000`.
///
/// Every built-in property also accepts the stylesheet-only global values
/// `inherit`, `initial`, and `unset`. `inherit` copies the parent's final
/// effective value, including an inline prop override. `initial` restores the
/// property's Nestix default. `unset` inherits font family, size, weight,
/// style, and text color, and restores the initial value for all other
/// properties. Custom properties keep these words as ordinary string values.
/// Global values are not accepted by inline props or by `$()` insertions.
///
/// Existing Rust values can be inserted with `$()`. Inserted built-in values
/// must already have the expected Rust type. Wrap the stylesheet in
/// `computed!` explicitly when it should update reactively.
///
/// ```rust,ignore
/// # use nestix_native_core::*;
/// let bg_color = nestix::create_state(Color::WHITE);
///
/// let styles = nestix::computed!([bg_color] || style! {
///     .counter {
///         bg_color: $(bg_color.get());
///         width: $(Dimension::from(240.0));
///         --label: $(format!("count-{}", 1));
///     }
/// });
/// ```
///
/// At the top level, `$()` embeds an existing `StyleSheet` at that source
/// position.
///
/// ```rust,ignore
/// # use nestix_native_core::*;
/// let base = style! {
///     .counter {
///         bg_color: blue;
///     }
/// };
///
/// let styles = style! {
///     .counter {
///         width: 240px;
///     }
///
///     $(base)
/// };
/// ```
///
/// Rules may be nested. A plain nested selector is a descendant of its parent,
/// `&` refers to the parent selector, and a leading combinator relates the
/// nested selector directly to its parent. Nesting is recursive, and
/// comma-separated parent and child selectors are expanded as a Cartesian
/// product.
///
/// ```rust,ignore
/// # use nestix_native_core::*;
/// let styles = style! {
///     .panel, .dialog {
///         padding: 12px;
///
///         &.selected {
///             bg_color: blue;
///         }
///
///         > .title {
///             font_weight: bold;
///         }
///
///         .actions {
///             > .button {
///                 margin_left: 8px;
///             }
///         }
///     }
/// };
/// ```
///
/// Nested selectors support `&`, implicit descendants, and leading `>`,
/// `>>`, `+`, and `~` combinators. Because Rust token streams do not preserve
/// whitespace, use `>>` when an explicit descendant combinator is needed
/// inside a selector.
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    style::style(input)
}
