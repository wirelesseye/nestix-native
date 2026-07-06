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
///         grow: 1;
///         align_self: center;
///         --accent_token: primary;
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
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    style::style(input)
}
