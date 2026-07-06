use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = Path::new(&manifest_dir).parent().unwrap();
    let packages_dir = workspace_dir.join(".packages");
    let foundation_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.Foundation")
        .join("1.8.260527000");
    let winui_metadata_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.WinUI")
        .join("1.8.260528001")
        .join("metadata");
    let foundation_metadata_dir = foundation_dir.join("metadata");
    let ixp_metadata_dir = packages_dir
        .join("Microsoft.WindowsAppSDK.InteractiveExperiences")
        .join("1.8.260525001")
        .join("metadata")
        .join("10.0.18362.0");

    let mut metadata_files = Vec::new();
    collect_winmds(&winui_metadata_dir, &mut metadata_files);
    collect_winmds(&foundation_metadata_dir, &mut metadata_files);
    collect_winmds(&ixp_metadata_dir, &mut metadata_files);

    if metadata_files.is_empty() {
        panic!(
            "Windows App SDK metadata not found under {}",
            packages_dir.display()
        );
    }

    for file in &metadata_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("bindings.rs");
    let out = out.to_string_lossy().into_owned();

    let mut args = vec!["--in".to_string(), "default".to_string()];
    args.extend(
        metadata_files
            .iter()
            .map(|file| file.to_string_lossy().into_owned()),
    );
    args.extend([
        "--out".to_string(),
        out,
        "--no-allow".to_string(),
        "--filter".to_string(),
        "Microsoft.UI.Xaml.Application".to_string(),
        "Microsoft.UI.Xaml.ApplicationInitializationCallback".to_string(),
        "Microsoft.UI.Xaml.DependencyObject".to_string(),
        "Microsoft.UI.Xaml.Window".to_string(),
        "Microsoft.UI.Xaml.Thickness".to_string(),
        "Microsoft.UI.Xaml.HorizontalAlignment".to_string(),
        "Microsoft.UI.Xaml.VerticalAlignment".to_string(),
        "Microsoft.UI.Xaml.Controls.Button".to_string(),
        "Microsoft.UI.Xaml.Controls.ContentControl".to_string(),
        "Microsoft.UI.Xaml.Controls.Control".to_string(),
        "Microsoft.UI.Xaml.Controls.Panel".to_string(),
        "Microsoft.UI.Xaml.Controls.Primitives.ButtonBase".to_string(),
        "Microsoft.UI.Xaml.Controls.StackPanel".to_string(),
        "Microsoft.UI.Xaml.Controls.TextBlock".to_string(),
        "Microsoft.UI.Xaml.Controls.UIElementCollection".to_string(),
        "Microsoft.UI.Xaml.Controls.Orientation".to_string(),
        "Microsoft.UI.Xaml.UIElement".to_string(),
        "Microsoft.UI.Xaml.FrameworkElement".to_string(),
        "Microsoft.UI.Xaml.RoutedEventArgs".to_string(),
        "Microsoft.UI.Xaml.RoutedEventHandler".to_string(),
    ]);

    let warnings = windows_bindgen::bindgen(args);
    if !warnings.is_empty() {
        println!(
            "cargo:warning=windows-bindgen skipped {} WinUI methods whose dependency types were not included",
            warnings.len()
        );
    }

    link_windows_app_runtime_bootstrap(&foundation_dir);
}

fn collect_winmds(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_winmds(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "winmd")
        {
            files.push(path);
        }
    }
}

fn link_windows_app_runtime_bootstrap(foundation_dir: &Path) {
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86" => "x86",
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unsupported Windows App SDK target architecture: {other}"),
    };
    let lib_dir = foundation_dir.join("lib").join("native").join(arch);
    let runtime_dir = foundation_dir
        .join("runtimes")
        .join(format!("win-{arch}"))
        .join("native");
    let bootstrap_dll = runtime_dir.join("Microsoft.WindowsAppRuntime.Bootstrap.dll");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=Microsoft.WindowsAppRuntime.Bootstrap");
    println!("cargo:rerun-if-changed={}", bootstrap_dll.display());

    if let Some(target_dir) = target_profile_dir() {
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(
            &bootstrap_dll,
            target_dir.join("Microsoft.WindowsAppRuntime.Bootstrap.dll"),
        )
        .unwrap_or_else(|err| {
            panic!(
                "failed to copy {} to {}: {err}",
                bootstrap_dll.display(),
                target_dir.display()
            )
        });
    }
}

fn target_profile_dir() -> Option<std::path::PathBuf> {
    let out_dir = std::path::PathBuf::from(env::var("OUT_DIR").ok()?);
    let profile = env::var("PROFILE").ok()?;
    let mut ancestors = out_dir.ancestors();
    while let Some(path) = ancestors.next() {
        if path
            .file_name()
            .is_some_and(|name| name == profile.as_str())
        {
            return Some(path.to_path_buf());
        }
    }
    None
}
