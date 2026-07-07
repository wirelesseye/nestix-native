use std::{env, fs, path::Path};

pub fn embed_manifest() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is not set");
    let manifest = Path::new(&out_dir).join("nestix-native-win32.app.manifest");
    fs::write(&manifest, include_bytes!("../app.manifest")).unwrap_or_else(|err| {
        panic!(
            "failed to write {} for Win32 manifest embedding: {err}",
            manifest.display()
        )
    });

    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
