use proc_macro_crate::FoundCrate;
use syn::{Path, parse_quote, parse_str};

pub fn nestix_native_path() -> Path {
    let found_crate = proc_macro_crate::crate_name("nestix-native")
        .or(proc_macro_crate::crate_name("nestix-native-core"))
        .unwrap();
    match found_crate {
        FoundCrate::Itself => {
            parse_quote!(crate)
        }
        FoundCrate::Name(name) => parse_str(&name).unwrap(),
    }
}
