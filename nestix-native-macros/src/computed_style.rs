use crate::utils::{nestix_native_path, nestix_path};
use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::quote;

pub fn computed_style(input: TokenStream) -> TokenStream {
    let nestix_path = nestix_path();
    let nestix_native_path = nestix_native_path();
    let mut tokens = TokenStream2::from(input).into_iter();

    let (captures, style) = match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => {
            let captures = group.stream();
            (Some(captures), tokens.collect())
        }
        Some(first) => (None, std::iter::once(first).chain(tokens).collect()),
        None => (None, TokenStream2::new()),
    };

    let closure = if let Some(captures) = captures {
        quote!([#captures] || #nestix_native_path::style! { #style })
    } else {
        quote!(|| #nestix_native_path::style! { #style })
    };

    quote!(#nestix_path::computed!(#closure)).into()
}
