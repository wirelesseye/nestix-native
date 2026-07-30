use std::{
    cell::RefCell,
    path::{Component, Path, PathBuf},
    rc::Rc,
};

use cargo_packager_resource_resolver::{PackageFormat, resources_dir};
use gtk4::{gio, prelude::*};
use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    JavaScriptEvaluator, StyleContext, WebViewBridgeScriptContext, WebViewDevToolsError,
    WebViewPresenter, WebViewProps, WebViewRegistration, WebViewSource, dpi::LogicalSize,
    matched_style, resolved_view_style,
};
use webkit6::{
    Settings, UserContentInjectedFrames, UserContentManager, UserScript, UserScriptInjectionTime,
    WebView as WebKitWebView, prelude::*,
};

use crate::layout::mount_leaf_with_intrinsic_size;

/// GTK4 web view backed by WebKitGTK.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) {
    require_visual_mount!(element, WebView);
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__gtk4_WebView"];

    let bridge = props.bridge.get();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style_props = resolved_view_style(matched, &props.view);

    let settings = Settings::new();
    settings.set_enable_developer_extras(props.inspectable.get());
    settings.set_allow_file_access_from_file_urls(true);

    // Each view owns its manager so registered bridge names and callbacks never
    // collide with another WebView or DomSurface.
    let content_manager = UserContentManager::new();
    let bridge_handler = bridge.as_ref().map(|bridge| {
        let handler_name = bridge.message_channel_name().to_string();
        assert!(
            content_manager.register_script_message_handler(&handler_name, None),
            "failed to register WebKitGTK message handler {handler_name:?}"
        );

        let message_bridge = bridge.clone();
        let signal_id = content_manager.connect_script_message_received(
            Some(&handler_name),
            move |_, value| {
                if value.is_string() {
                    message_bridge.receive_message(value.to_str().as_str());
                }
            },
        );

        let escaped_name = javascript_single_quoted_string(&handler_name);
        let post_message_expression = format!(
            "message => window.webkit.messageHandlers['{escaped_name}'].postMessage(message)"
        );
        if let Some(source) = bridge.initialization_script(WebViewBridgeScriptContext {
            post_message_expression: &post_message_expression,
        }) {
            content_manager.add_script(&UserScript::new(
                &source,
                UserContentInjectedFrames::TopFrame,
                UserScriptInjectionTime::Start,
                &[],
                &[],
            ));
        }

        (handler_name, signal_id)
    });

    let web_view = WebKitWebView::builder()
        .settings(&settings)
        .user_content_manager(&content_manager)
        .build();
    web_view.set_size_request(300, 150);
    let opaque_background = web_view.background_color();

    if let Some(bridge) = &bridge {
        let weak_web_view = web_view.downgrade();
        let evaluator: JavaScriptEvaluator = Rc::new(move |script| {
            let Some(web_view) = weak_web_view.upgrade() else {
                return;
            };
            web_view.evaluate_javascript(script, None, None, gio::Cancellable::NONE, |_| {});
        });
        bridge.attach(evaluator);
    }

    let node_id = mount_leaf_with_intrinsic_size(
        element,
        web_view.upcast_ref(),
        style_props.into_readonly(),
        &props.view,
        create_state(0usize).0.into_readonly(),
        LogicalSize::new(300.0, 150.0),
    );
    let tree_context = element
        .context::<nestix_native_core::TreeContext>()
        .unwrap();
    let (mapped, set_mapped) = create_state(web_view.is_mapped());
    web_view.connect_map(closure!(
        [mapped] | _ | {
            set_mapped.set(true);
        }
    ));
    let last_loaded_source = Rc::new(RefCell::new(None::<WebViewSource>));
    scoped_effect!(
        [
            web_view,
            props.source,
            mapped,
            tree_context,
            last_loaded_source
        ] || {
            let Some(layout) = tree_context.layout(node_id) else {
                return;
            };
            if !mapped.get() || layout.size.width <= 0.0 || layout.size.height <= 0.0 {
                return;
            }
            let source = source.get();
            if last_loaded_source.borrow().as_ref() != Some(&source) {
                last_loaded_source.replace(Some(source.clone()));
                load_source(&web_view, source);
            }
        }
    );

    scoped_effect!(
        [settings, props.inspectable] || {
            settings.set_enable_developer_extras(inspectable.get());
        }
    );

    scoped_effect!(
        [web_view, props.transparent, opaque_background] || {
            web_view.set_background_color(if transparent.get() {
                &gtk4::gdk::RGBA::TRANSPARENT
            } else {
                &opaque_background
            });
        }
    );

    let controller_registration = Rc::new(RefCell::new(None::<WebViewRegistration>));
    scoped_effect!(
        [
            web_view,
            props.inspectable,
            props.controller,
            controller_registration
        ] || {
            controller_registration.borrow_mut().take();
            let weak_web_view = web_view.downgrade();
            controller_registration
                .borrow_mut()
                .replace(controller.get().bind(WebViewPresenter {
                    open_dev_tools: callback!(
                        [weak_web_view, inspectable] || {
                            if !inspectable.get() {
                                return Err(WebViewDevToolsError::NotInspectable);
                            }
                            let web_view = weak_web_view
                                .upgrade()
                                .ok_or(WebViewDevToolsError::NotMounted)?;
                            let inspector = web_view.inspector().ok_or_else(|| {
                                WebViewDevToolsError::Unsupported(
                                    "WebKitGTK did not provide a web inspector".to_string(),
                                )
                            })?;
                            inspector.show();
                            Ok(())
                        }
                    ),
                }));
        }
    );

    element.on_unmount(closure!(
        [controller_registration] || {
            controller_registration.borrow_mut().take();
        }
    ));
    if let (Some(bridge), Some((handler_name, signal_id))) = (bridge, bridge_handler) {
        let signal_id = Rc::new(RefCell::new(Some(signal_id)));
        element.on_unmount(move || {
            if let Some(signal_id) = signal_id.borrow_mut().take() {
                content_manager.disconnect(signal_id);
            }
            content_manager.unregister_script_message_handler(&handler_name, None);
            bridge.detach();
        });
    }
}

fn load_source(web_view: &WebKitWebView, source: WebViewSource) {
    match source {
        WebViewSource::Url(url) => web_view.load_uri(&url),
        WebViewSource::Html { html, base_url } => {
            web_view.load_html(&html, base_url.as_deref());
        }
        WebViewSource::Resource {
            path,
            development_path,
        } => {
            let document = resolve_document_resource(&path, development_path.as_deref());
            let uri = gio::File::for_path(document).uri();
            web_view.load_uri(&uri);
        }
    }
}

fn resolve_document_resource(path: &Path, development_path: Option<&Path>) -> PathBuf {
    validate_resource_path(path);
    let packaged = packaged_resource_root().map(|root| root.join(path));
    resolve_resource_candidates(path, packaged.as_deref(), development_path)
}

fn resolve_resource_candidates(
    path: &Path,
    packaged: Option<&Path>,
    development_path: Option<&Path>,
) -> PathBuf {
    if let Some(candidate) = packaged
        && let Ok(candidate) = candidate.canonicalize()
        && candidate.is_file()
    {
        return candidate;
    }
    if let Some(candidate) = development_path
        && let Ok(candidate) = candidate.canonicalize()
        && candidate.is_file()
    {
        return candidate;
    }
    panic!(
        "WebView resource {path:?} was not found; packaged location: {}; development location: {}",
        packaged.map_or_else(
            || "<package resource directory unavailable>".into(),
            |path| format!("{path:?}")
        ),
        development_path.map_or_else(|| "<not provided>".into(), |path| format!("{path:?}")),
    );
}

fn packaged_resource_root() -> Option<PathBuf> {
    let format = if std::env::var_os("APPDIR").is_some() {
        PackageFormat::AppImage
    } else {
        // cargo-packager uses the same /usr/lib/<binary-name> resource
        // directory for Debian and Pacman packages.
        PackageFormat::Deb
    };
    resources_dir(format).ok()
}

fn validate_resource_path(path: &Path) {
    assert!(
        !path.as_os_str().is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "WebView resource paths must be non-empty relative paths without `..`: {path:?}"
    );
}

fn javascript_single_quoted_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_paths_reject_unsafe_values() {
        for path in ["", "../index.html", "web/../index.html", "/index.html"] {
            assert!(
                std::panic::catch_unwind(|| validate_resource_path(Path::new(path))).is_err(),
                "accepted unsafe resource path {path:?}"
            );
        }
        validate_resource_path(Path::new("web/index.html"));
    }

    #[test]
    fn javascript_handler_names_are_escaped() {
        assert_eq!(
            javascript_single_quoted_string("surface'\\\n"),
            "surface\\'\\\\\\n"
        );
    }
}
