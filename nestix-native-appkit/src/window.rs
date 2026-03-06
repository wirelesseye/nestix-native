use std::rc::Rc;

use nestix::{
    Element, PropValue, Readonly, Shared, callback, component, components::ContextProvider,
    create_state, effect, layout,
};
use nestix_native_core::{
    WindowProps,
    dpi::{self, LogicalSize},
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSSize, NSString};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};

use crate::contexts::{ParentContext, TreeContext};

pub struct WindowContext {
    pub ns_window: Retained<NSWindow>,
    pub scale_factor: Readonly<f64>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let scale_factor = create_state(1.0);

    let ns_window = unsafe { NSWindow::new(mtm) };

    let window_context = Rc::new(WindowContext {
        ns_window: ns_window.clone(),
        scale_factor: scale_factor.clone().into_readonly(),
    });
    let tree_context = Rc::new(TreeContext::new());

    let window_delegate = WindowDelegate::new(
        mtm,
        WindowState {
            tree_context: tree_context.clone(),
            on_resize: props.on_resize.clone(),
        },
    );
    let style_mask = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;
    ns_window.setStyleMask(style_mask);
    ns_window.makeKeyAndOrderFront(None);
    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*window_delegate)));

    scale_factor.set(ns_window.backingScaleFactor());

    element.provide_handle(ns_window.as_ref() as *const NSObject);

    effect!(
        [ns_window, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            ns_window.setTitle(&ns_string);
        }
    );

    effect!(
        [ns_window, props.width, props.height] || {
            ns_window.setContentSize(NSSize::new(width.get(), height.get()));
        }
    );

    ns_window.center();

    layout! {
        ContextProvider<WindowContext>(
            .value = window_context.clone(),
        ) {
            ContextProvider<TreeContext>(
                .value = tree_context.clone(),
            ) {
                ContextProvider<ParentContext>(
                    .value = ParentContext {
                        add_child: Some(callback!([ns_window] |object: &NSObject, child_node: Option<NodeId>| {
                            let view = object.downcast_ref::<NSView>().unwrap();
                            ns_window.setContentView(Some(view));
                            tree_context.set_root_node(child_node);

                            let size = view.frame().size;
                            if let Some(child_node) = child_node {
                                tree_context.update_style(child_node, |prev| Style {
                                    size: Size {
                                        width: Dimension::from_length(size.width as f32),
                                        height: Dimension::from_length(size.height as f32)
                                    },
                                    ..prev
                                });
                            }
                        })),
                        remove_child: None,
                        parent_node: None,
                    }
                ) {
                    $(props.children.get())
                }
            }
        }
    }
}

struct WindowState {
    tree_context: Rc<TreeContext>,
    on_resize: PropValue<Option<Shared<dyn Fn(dpi::Size)>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WindowDelegate"]
    #[ivars = WindowState]
    struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, notification: &NSNotification) {
            let window = notification
                .object()
                .unwrap()
                .downcast::<NSWindow>()
                .unwrap();
            let size = window.contentView().unwrap().frame().size;

            let tree_context = &self.ivars().tree_context;
            if let Some(root_node) = tree_context.root_node() {
                tree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: Dimension::from_length(size.width as f32),
                        height: Dimension::from_length(size.height as f32),
                    },
                    ..prev
                });
                tree_context.update();
            }

            if let Some(on_resize) = self.ivars().on_resize.get() {
                on_resize(dpi::Size::Logical(LogicalSize::new(
                    size.width,
                    size.height,
                )));
            }
        }
    }
);

impl WindowDelegate {
    fn new(mtm: MainThreadMarker, state: WindowState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
