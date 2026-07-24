use proc_macro_crate::FoundCrate;
use syn::{Path, parse_quote, parse_str};

pub fn nestix_path() -> Path {
    crate_path("nestix").unwrap()
}

pub fn nestix_native_path() -> Path {
    crate_path("nestix-native")
        .or_else(|| crate_path("nestix-native-core"))
        .unwrap()
}

fn crate_path(name: &str) -> Option<Path> {
    proc_macro_crate::crate_name(name)
        .ok()
        .map(found_crate_path)
}

fn found_crate_path(found_crate: FoundCrate) -> Path {
    match found_crate {
        FoundCrate::Itself => parse_quote!(crate),
        FoundCrate::Name(name) => parse_str(&name).unwrap(),
    }
}
