use std::rc::Rc;

use nestix::{
    Element, Layout, PropValue, Readonly, Shared, State, callback, closure, component,
    components::ContextProvider, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    StyleScope, TreeContext, WindowProps,
    dpi::{self, LogicalSize},
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSMenu, NSToolbar, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSSize, NSString};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};

use crate::{contexts::ParentContext, root::RootContext};

pub struct WindowContext {
    pub ns_window: Retained<NSWindow>,
    pub scale_factor: Readonly<f64>,
    pub(crate) menu: State<Option<Retained<NSMenu>>>,
    pub(crate) toolbar: State<Option<Retained<NSToolbar>>>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__appkit_Window"];

    let mtm = MainThreadMarker::new().unwrap();
    let scale_factor = create_state(1.0);
    let menu = create_state(None::<Retained<NSMenu>>);
    let toolbar = create_state(None::<Retained<NSToolbar>>);
    let root_context = element.context::<RootContext>().unwrap();

    let ns_window = unsafe { NSWindow::new(mtm) };

    let window_context = Rc::new(WindowContext {
        ns_window: ns_window.clone(),
        scale_factor: scale_factor.clone().into_readonly(),
        menu: menu.clone(),
        toolbar,
    });
    let tree_context = Rc::new(TreeContext::new());

    let window_delegate = WindowDelegate::new(
        mtm,
        WindowState {
            tree_context: tree_context.clone(),
            on_resize: props.on_resize.clone(),
            menu,
            active_window_menu: root_context.active_window_menu.clone(),
        },
    );
    let style_mask = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;
    ns_window.setStyleMask(style_mask);
    ns_window.makeKeyAndOrderFront(None);
    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*window_delegate)));

    // NSWindow does not retain its delegate.
    element.on_unmount(closure!(
        [ns_window, window_delegate] || {
            ns_window.setDelegate(None);
            let _ = &window_delegate;
        }
    ));

    scale_factor.set(ns_window.backingScaleFactor());

    element.provide_handle(ns_window.as_ref() as *const NSObject);

    scoped_effect!(
        element,
        [ns_window, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            ns_window.setTitle(&ns_string);
        }
    );

    scoped_effect!(
        element,
        [ns_window, props.width, props.height] || {
            ns_window.setContentSize(NSSize::new(width.get(), height.get()));
        }
    );

    ns_window.center();

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
                    ContextProvider<ParentContext>(ParentContext {
                        add_child: Some(callback!([ns_window, tree_context] |object: &NSObject, child_node: Option<NodeId>| {
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
                                tree_context.refresh();
                            }
                        })),
                        insert_child: None,
                        remove_child: Some(callback!([ns_window] |_: &NSObject, _: Option<NodeId>| {
                            ns_window.setContentView(None);
                            tree_context.set_root_node(None);
                        })),
                        parent_node: None,
                    }) {
                        $(props.children.clone().map(|element| Layout::from(element.clone())))
                    }
                }
            }
        }
    }
}

struct WindowState {
    tree_context: Rc<TreeContext>,
    on_resize: PropValue<Option<Shared<dyn Fn(dpi::Size)>>>,
    menu: State<Option<Retained<NSMenu>>>,
    active_window_menu: State<Option<Retained<NSMenu>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WindowDelegate"]
    #[ivars = WindowState]
    struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _: &NSNotification) {
            self.ivars().active_window_menu.set(self.ivars().menu.get());
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _: &NSNotification) {
            let menu = self.ivars().menu.get();
            let active = self.ivars().active_window_menu.get();
            if same_menu(&active, &menu) {
                self.ivars().active_window_menu.set(None);
            }
        }

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
                tree_context.refresh();
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

fn same_menu(left: &Option<Retained<NSMenu>>, right: &Option<Retained<NSMenu>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => std::ptr::eq::<NSMenu>(left.as_ref(), right.as_ref()),
        (None, None) => true,
        _ => false,
    }
}

impl WindowDelegate {
    fn new(mtm: MainThreadMarker, state: WindowState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
