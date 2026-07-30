use std::{
    cell::RefCell,
    ffi::c_void,
    path::{Component, Path, PathBuf},
    ptr,
    rc::{Rc, Weak},
    sync::{
        Once,
        atomic::{AtomicU64, Ordering},
    },
};

use nestix::{Element, callback, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    JavaScriptEvaluator, StyleContext, WebViewBridge, WebViewBridgeScriptContext,
    WebViewDevToolsError, WebViewPresenter, WebViewProps, WebViewRegistration, WebViewSource,
    dpi::LogicalSize, matched_style, resolved_view_style,
};
use webview2_com::{
    AddScriptToExecuteOnDocumentCreatedCompletedHandler, CoTaskMemPWSTR,
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
    ExecuteScriptCompletedHandler, Microsoft::Web::WebView2::Win32::*,
    WebMessageReceivedEventHandler,
};
use windows::{
    Win32::{
        Foundation::{E_POINTER, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
    core::{Interface, PCWSTR, PWSTR, w},
};

use crate::{contexts::ParentContext, native_control};

const RESOURCE_HOST_PREFIX: &str = "nestix-resource-";
static NEXT_RESOURCE_HOST: AtomicU64 = AtomicU64::new(1);

struct WebViewState {
    controller: Option<ICoreWebView2Controller>,
    web_view: Option<ICoreWebView2>,
    message_token: Option<i64>,
    script_id: Option<String>,
}

impl WebViewState {
    fn resize(&self, hwnd: HWND) {
        let Some(controller) = &self.controller else {
            return;
        };
        let mut bounds = RECT::default();
        unsafe {
            if GetClientRect(hwnd, &mut bounds).is_ok() {
                let _ = controller.SetBounds(bounds);
            }
        }
    }
}

impl Drop for WebViewState {
    fn drop(&mut self) {
        unsafe {
            if let Some(web_view) = &self.web_view {
                if let Some(token) = self.message_token.take() {
                    let _ = web_view.remove_WebMessageReceived(token);
                }
                if let Some(id) = self.script_id.take() {
                    let id = CoTaskMemPWSTR::from(id.as_str());
                    let _ =
                        web_view.RemoveScriptToExecuteOnDocumentCreated(*id.as_ref().as_pcwstr());
                }
            }
            if let Some(controller) = self.controller.take() {
                let _ = controller.Close();
            }
        }
    }
}

fn web_view_classname() -> PCWSTR {
    const NAME: PCWSTR = w!("NestixNativeWebView");
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        RegisterClassW(&WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: NAME,
            lpfnWndProc: Some(web_view_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            ..Default::default()
        });
    });
    NAME
}

unsafe fn state_for_window(hwnd: HWND) -> Option<Rc<RefCell<WebViewState>>> {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<WebViewState>;
    if pointer.is_null() {
        return None;
    }
    unsafe { Rc::increment_strong_count(pointer) };
    Some(unsafe { Rc::from_raw(pointer) })
}

extern "system" fn web_view_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                LRESULT(1)
            }
            WM_SIZE => {
                if let Some(state) = state_for_window(hwnd) {
                    state.borrow().resize(hwnd);
                }
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let pointer =
                    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<WebViewState>;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if !pointer.is_null() {
                    drop(Rc::from_raw(pointer));
                }
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

/// Displays web content in a child WebView2 control.
#[component]
pub fn WebView(props: &WebViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__WebView", "__win32_WebView"];
    let parent = element.context::<ParentContext>().unwrap();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let style = resolved_view_style(matched, &props.view);
    let bridge = props.bridge.get();
    let state = Rc::new(RefCell::new(WebViewState {
        controller: None,
        web_view: None,
        message_token: None,
        script_id: None,
    }));
    let instance = unsafe { GetModuleHandleW(None).unwrap() };
    let raw_state = Rc::into_raw(state.clone()) as *mut c_void;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            web_view_classname(),
            None,
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            Some(parent.surface.hwnd()),
            None,
            Some(instance.into()),
            Some(raw_state),
        )
        .unwrap()
    };

    let (controller, web_view) = create_web_view(hwnd);
    state.borrow_mut().controller = Some(controller.clone());
    state.borrow_mut().web_view = Some(web_view.clone());
    state.borrow().resize(hwnd);

    if let Some(bridge) = &bridge {
        install_bridge(&state, &web_view, bridge.clone());
        let weak_state: Weak<RefCell<WebViewState>> = Rc::downgrade(&state);
        let evaluator: JavaScriptEvaluator = Rc::new(move |script| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let Some(web_view) = state.borrow().web_view.clone() else {
                return;
            };
            execute_script(&web_view, script);
        });
        bridge.attach(evaluator);
    }

    scoped_effect!(
        [state, props.source] || {
            if let Some(web_view) = state.borrow().web_view.clone() {
                load_source(&web_view, source.get());
            }
        }
    );

    scoped_effect!(
        [state, props.inspectable] || {
            if let Some(web_view) = state.borrow().web_view.clone() {
                let settings = unsafe {
                    web_view
                        .Settings()
                        .expect("failed to access WebView2 settings")
                };
                unsafe {
                    settings
                        .SetAreDevToolsEnabled(inspectable.get())
                        .expect("failed to update WebView2 developer tools")
                };
            }
        }
    );

    let controller_registration = Rc::new(RefCell::new(None::<WebViewRegistration>));
    scoped_effect!(
        [
            state,
            props.inspectable,
            props.controller,
            controller_registration
        ] || {
            controller_registration.borrow_mut().take();
            let weak_state = Rc::downgrade(&state);
            controller_registration
                .borrow_mut()
                .replace(controller.get().bind(WebViewPresenter {
                    open_dev_tools: callback!(
                        [weak_state, inspectable] || {
                            if !inspectable.get() {
                                return Err(WebViewDevToolsError::NotInspectable);
                            }
                            let state = weak_state
                                .upgrade()
                                .ok_or(WebViewDevToolsError::NotMounted)?;
                            let web_view = state
                                .borrow()
                                .web_view
                                .clone()
                                .ok_or(WebViewDevToolsError::NotMounted)?;
                            unsafe { web_view.OpenDevToolsWindow() }
                                .map_err(|error| WebViewDevToolsError::Backend(error.to_string()))
                        }
                    ),
                }));
        }
    );

    scoped_effect!(
        [state, props.transparent] || {
            let controller = state.borrow().controller.clone();
            if let Some(controller) = controller {
                let controller2: ICoreWebView2Controller2 = controller
                    .cast()
                    .expect("WebView2 controller does not support background colors");
                let color = if transparent.get() {
                    COREWEBVIEW2_COLOR {
                        A: 0,
                        R: 0,
                        G: 0,
                        B: 0,
                    }
                } else {
                    COREWEBVIEW2_COLOR {
                        A: 255,
                        R: 255,
                        G: 255,
                        B: 255,
                    }
                };
                unsafe {
                    controller2
                        .SetDefaultBackgroundColor(color)
                        .expect("failed to update WebView2 background")
                };
            }
        }
    );

    native_control::mount(
        element,
        hwnd,
        style,
        &props.view,
        create_state(LogicalSize::new(300.0, 150.0)).into_readonly(),
    );
    if let Some(bridge) = bridge {
        element.on_unmount(move || bridge.detach());
    }
    element.on_unmount(closure!(
        [controller_registration] || {
            controller_registration.borrow_mut().take();
        }
    ));
}

fn create_web_view(hwnd: HWND) -> (ICoreWebView2Controller, ICoreWebView2) {
    let (environment_tx, environment_rx) = std::sync::mpsc::channel();
    CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
        Box::new(|handler| unsafe {
            CreateCoreWebView2Environment(&handler).map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |result, environment| {
            result?;
            environment_tx
                .send(environment.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                .unwrap();
            Ok(())
        }),
    )
    .expect("failed to start WebView2 environment creation");
    let environment = webview2_com::wait_with_pump(environment_rx)
        .expect("WebView2 environment callback failed")
        .expect("WebView2 environment was not created");

    let (controller_tx, controller_rx) = std::sync::mpsc::channel();
    CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            environment
                .CreateCoreWebView2Controller(hwnd, &handler)
                .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |result, controller| {
            result?;
            controller_tx
                .send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                .unwrap();
            Ok(())
        }),
    )
    .expect("failed to start WebView2 controller creation");
    let controller = webview2_com::wait_with_pump(controller_rx)
        .expect("WebView2 controller callback failed")
        .expect("WebView2 controller was not created");
    unsafe {
        controller
            .SetIsVisible(true)
            .expect("failed to show WebView2");
        let web_view = controller
            .CoreWebView2()
            .expect("failed to access WebView2");
        (controller, web_view)
    }
}

fn install_bridge(
    state: &Rc<RefCell<WebViewState>>,
    web_view: &ICoreWebView2,
    bridge: Rc<dyn WebViewBridge>,
) {
    let message_bridge = bridge.clone();
    let handler = WebMessageReceivedEventHandler::create(Box::new(move |_sender, args| {
        if let Some(args) = args {
            let mut message = PWSTR(ptr::null_mut());
            unsafe {
                if args.TryGetWebMessageAsString(&mut message).is_ok() {
                    message_bridge.receive_message(&CoTaskMemPWSTR::from(message).to_string());
                }
            }
        }
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        web_view
            .add_WebMessageReceived(&handler, &mut token)
            .expect("failed to register WebView bridge message handler")
    };
    state.borrow_mut().message_token = Some(token);

    let script = bridge
        .initialization_script(WebViewBridgeScriptContext {
            post_message_expression: "message => window.chrome.webview.postMessage(message)",
        })
        .unwrap_or_default();
    let (tx, rx) = std::sync::mpsc::channel();
    let web_view_for_add = web_view.clone();
    AddScriptToExecuteOnDocumentCreatedCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            let script = CoTaskMemPWSTR::from(script.as_str());
            web_view_for_add
                .AddScriptToExecuteOnDocumentCreated(*script.as_ref().as_pcwstr(), &handler)
                .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |result, id| {
            result?;
            tx.send(id).unwrap();
            Ok(())
        }),
    )
    .expect("failed to install WebView bridge initialization script");
    state.borrow_mut().script_id =
        Some(webview2_com::wait_with_pump(rx).expect("WebView bridge script callback failed"));
}

fn execute_script(web_view: &ICoreWebView2, script: &str) {
    let script = CoTaskMemPWSTR::from(script);
    let handler = ExecuteScriptCompletedHandler::create(Box::new(|result, _| result));
    unsafe {
        web_view
            .ExecuteScript(*script.as_ref().as_pcwstr(), &handler)
            .expect("failed to execute WebView JavaScript")
    };
}

fn load_source(web_view: &ICoreWebView2, source: WebViewSource) {
    unsafe {
        match source {
            WebViewSource::Url(url) => {
                let url = CoTaskMemPWSTR::from(url.as_str());
                web_view
                    .Navigate(*url.as_ref().as_pcwstr())
                    .expect("WebView2 failed to navigate");
            }
            WebViewSource::Html { html, base_url } => {
                let html = base_url.map_or(html.clone(), |base| inject_base_url(&html, &base));
                let html = CoTaskMemPWSTR::from(html.as_str());
                web_view
                    .NavigateToString(*html.as_ref().as_pcwstr())
                    .expect("WebView2 failed to load HTML");
            }
            WebViewSource::Resource {
                path,
                development_path,
            } => {
                let document = resolve_document_resource(&path, development_path.as_deref());
                let root = document
                    .parent()
                    .expect("WebView resource must have a parent");
                let host = format!(
                    "{RESOURCE_HOST_PREFIX}{}.local",
                    NEXT_RESOURCE_HOST.fetch_add(1, Ordering::Relaxed)
                );
                let web_view3: ICoreWebView2_3 = web_view
                    .cast()
                    .expect("WebView2 does not support virtual host mappings");
                let host_wide = CoTaskMemPWSTR::from(host.as_str());
                let root_wide = CoTaskMemPWSTR::from(root.to_string_lossy().as_ref());
                web_view3
                    .SetVirtualHostNameToFolderMapping(
                        *host_wide.as_ref().as_pcwstr(),
                        *root_wide.as_ref().as_pcwstr(),
                        COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW,
                    )
                    .expect("failed to map WebView resource directory");
                let file =
                    percent_encode_path(document.file_name().unwrap().to_string_lossy().as_ref());
                let url = CoTaskMemPWSTR::from(format!("https://{host}/{file}").as_str());
                web_view
                    .Navigate(*url.as_ref().as_pcwstr())
                    .expect("WebView2 failed to navigate to resource");
            }
        }
    }
}

fn inject_base_url(html: &str, base_url: &str) -> String {
    let escaped = base_url
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let base = format!(r#"<base href="{escaped}">"#);
    if let Some(index) = html.to_ascii_lowercase().find("<head")
        && let Some(end) = html[index..].find('>')
    {
        let insertion = index + end + 1;
        return format!("{}{}{}", &html[..insertion], base, &html[insertion..]);
    }
    format!("{base}{html}")
}

fn resolve_document_resource(path: &Path, development_path: Option<&Path>) -> PathBuf {
    validate_resource_path(path);
    let package_root = current_package_path();
    let packaged = package_root.as_ref().map(|root| root.join(path));
    if let Some(candidate) = &packaged
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
        packaged.as_ref().map_or_else(
            || "<application is unpackaged>".into(),
            |p| format!("{p:?}")
        ),
        development_path.map_or_else(|| "<not provided>".into(), |p| format!("{p:?}")),
    );
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

fn percent_encode_path(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn current_package_path() -> Option<PathBuf> {
    const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;
    unsafe extern "system" {
        fn GetCurrentPackagePath(path_length: *mut u32, path: PWSTR) -> i32;
    }
    unsafe {
        let mut length = 0;
        let result = GetCurrentPackagePath(&mut length, PWSTR::null());
        if result == APPMODEL_ERROR_NO_PACKAGE {
            return None;
        }
        assert_eq!(
            result, 122,
            "GetCurrentPackagePath size query failed with error {result}"
        );
        let mut buffer = vec![0u16; length as usize];
        let result = GetCurrentPackagePath(&mut length, PWSTR(buffer.as_mut_ptr()));
        assert_eq!(
            result, 0,
            "GetCurrentPackagePath failed with error {result}"
        );
        buffer.truncate(
            buffer
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(buffer.len()),
        );
        Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_paths_reject_traversal() {
        assert!(
            std::panic::catch_unwind(|| validate_resource_path(Path::new("../index.html")))
                .is_err()
        );
        assert!(std::panic::catch_unwind(|| validate_resource_path(Path::new(""))).is_err());
    }

    #[test]
    fn base_url_is_inserted_at_start_of_head() {
        assert_eq!(
            inject_base_url(
                "<html><head><title>x</title></head></html>",
                "https://example.test/a/"
            ),
            "<html><head><base href=\"https://example.test/a/\"><title>x</title></head></html>"
        );
    }
}
