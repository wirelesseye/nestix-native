mod style;
mod util;

use proc_macro::TokenStream;

/// Builds a Nestix Native stylesheet.
///
/// The macro accepts class selectors and typed built-in style properties. A
/// stylesheet with only literal values expands to a
/// `nestix_native_core::StyleSheet`.
///
/// ```rust
/// # use nestix_native_core::*;
/// let styles = style! {
///     .counter, .__Button {
///         bg-color: #FFFFFF;
///         width: 120px;
///         margin: 8px;
///         margin-left: 16px;
///         grow: 1;
///         align-self: center;
///         --accent-token: primary;
///     }
/// };
/// ```
///
/// Dimension literals must be `auto` or a pixel value such as `30px`.
/// Bare numeric dimensions such as `margin: 30;` are rejected. Built-in values
/// are parsed as their Rust types, while custom properties must use a `--`
/// prefix and are stored as strings.
///
/// Existing Rust values can be inserted with `$()`. Inserted built-in values
/// must already have the expected Rust type. If any inserted value is present,
/// the macro expands to `nestix::Computed<StyleSheet>`.
///
/// ```rust
/// # use nestix_native_core::*;
/// let bg_color = nestix::create_state(Color::WHITE);
///
/// let styles = style! {
///     [bg_color]
///     .counter {
///         bg-color: $(bg_color.get());
///         width: $(Dimension::from(240.0));
///         --label: $(format!("count-{}", 1));
///     }
/// };
/// ```
///
/// The optional leading capture list follows the same cloning semantics as
/// `nestix::closure!`. If `$()` is used without a capture list, the generated
/// computed stylesheet uses a `move` closure.
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    style::style(input)
}
