use std::rc::Rc;

use nestix::{
    Element, Layout, PropValue, Readonly, Shared, State, callback, closure, component,
    components::ContextProvider, computed, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, AnimationRuntime, Dimension as NativeDimension, StyleContext, StyleScope,
    TitleBarMode, TreeContext, WindowProps,
    dpi::{self, LogicalSize},
    matched_style, style_dimension,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject, sel,
};
use objc2_app_kit::{
    NSMenu, NSToolbar, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
    NSWindowTitleVisibility,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSSize, NSString, NSTimer};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};

use crate::{contexts::ParentContext, root::RootContext};

pub struct WindowContext {
    pub ns_window: Retained<NSWindow>,
    pub scale_factor: Readonly<f64>,
    pub animation: Rc<AnimationRuntime>,
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
    let style_context = element.context::<StyleContext>();

    let ns_window = unsafe { NSWindow::new(mtm) };
    let tree_context = Rc::new(TreeContext::new());
    let animation = Rc::new(AnimationRuntime::new());
    let animation_timer_target = AnimationTimerTarget::new(
        mtm,
        AnimationTimerState {
            animation: animation.clone(),
            tree_context: tree_context.clone(),
        },
    );
    let animation_timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            1.0 / 60.0,
            animation_timer_target.as_ref(),
            sel!(tick:),
            None,
            true,
        )
    };

    let window_context = Rc::new(WindowContext {
        ns_window: ns_window.clone(),
        scale_factor: scale_factor.clone().into_readonly(),
        animation: animation.clone(),
        menu: menu.clone(),
        toolbar,
    });

    let window_delegate = WindowDelegate::new(
        mtm,
        WindowState {
            tree_context: tree_context.clone(),
            on_resize: props.on_resize.clone(),
            on_close_requested: props.on_close_requested.clone(),
            menu,
            active_window_menu: root_context.active_window_menu.clone(),
        },
    );
    let style_mask = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;
    ns_window.setStyleMask(style_mask);
    apply_title_bar_mode(&ns_window, props.title_bar_mode.get());
    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*window_delegate)));

    // NSWindow does not retain its delegate.
    element.on_unmount(closure!(
        [
            ns_window,
            window_delegate,
            animation_timer,
            animation_timer_target
        ] || {
            animation_timer.invalidate();
            ns_window.setDelegate(None);
            ns_window.close();
            let _ = (&window_delegate, &animation_timer_target);
        }
    ));

    scale_factor.set(ns_window.backingScaleFactor());

    element.provide_handle(ns_window.as_ref() as *const NSObject);

    scoped_effect!(
        [ns_window, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            ns_window.setTitle(&ns_string);
        }
    );

    scoped_effect!(
        [root_context.ns_application, ns_window, props.visible] || {
            if visible.get() {
                ns_application.activate();
                ns_window.makeKeyAndOrderFront(None);
            } else {
                ns_window.orderOut(None);
            }
        }
    );

    scoped_effect!(
        [ns_window, props.title_bar_mode] || {
            apply_title_bar_mode(&ns_window, title_bar_mode.get());
        }
    );

    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_size = computed!(
        [style_props, props.width, props.height] || {
            let mut style = style_props.get().unwrap_or_default();
            style.width = Some(style_dimension(
                Some(&style),
                width.get().into(),
                NativeDimension::from(800),
                |style| style.width,
            ));
            style.height = Some(style_dimension(
                Some(&style),
                height.get().into(),
                NativeDimension::from(600),
                |style| style.height,
            ));
            Some(style)
        }
    );
    let animated_size = Rc::new(AnimatedStyle::new(animation, target_size.get()));
    let presented_size = animated_size.value();
    scoped_effect!(
        [animated_size, target_size, scale_factor] || {
            animated_size.set_target(target_size.get(), scale_factor.get());
        }
    );
    scoped_effect!(
        [ns_window, presented_size, scale_factor] || {
            let style = presented_size.get().unwrap_or_default();
            let current = ns_window
                .contentView()
                .map(|view| view.frame().size)
                .unwrap_or_else(|| NSSize::new(800.0, 600.0));
            ns_window.setContentSize(NSSize::new(
                logical_length(style.width, current.width, scale_factor.get()),
                logical_length(style.height, current.height, scale_factor.get()),
            ));
        }
    );

    ns_window.center();

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                StyleScope(
                    .class = props.class.clone(),
                    .default_classes = DEFAULT_CLASSES,
                    .effective_style = target_size,
                ) {
                    ContextProvider<ParentContext>(
                        ParentContext {
                            add_child: Some(callback!([ns_window, tree_context] |object: &NSObject,
                            child_node: Option<NodeId> | {
                                let view = object.downcast_ref::<NSView>().unwrap();
                                ns_window.setContentView(Some(view));
                                tree_context.set_root_node(child_node);
                                let size = view.frame().size;
                                if let Some(child_node) = child_node {
                                    tree_context.update_style(child_node, |prev| Style {
                                        size: Size {
                                            width: Dimension::from_length(size.width as f32),
                                            height: Dimension::from_length(size.height as f32),
                                        },
                                        ..prev
                                    });
                                    tree_context.refresh();
                                }
                            })),
                            insert_child: None,
                            remove_child: Some(callback!([ns_window] |_: &NSObject,
                            _: Option<NodeId> | {
                                ns_window.setContentView(None);
                                tree_context.set_root_node(None);
                            })),
                            parent_node: None
                        },
                    ) {
                        $(props.children.clone().map(|element| Layout::from(element.clone())))
                    }
                }
            }
        }
    }
}

fn logical_length(value: Option<NativeDimension>, fallback: f64, scale_factor: f64) -> f64 {
    match value {
        Some(NativeDimension::Length(value)) => value.to_logical::<f64>(scale_factor).0,
        Some(NativeDimension::Auto) | None => fallback,
    }
}

struct AnimationTimerState {
    animation: Rc<AnimationRuntime>,
    tree_context: Rc<TreeContext>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixAnimationTimerTarget"]
    #[ivars = AnimationTimerState]
    struct AnimationTimerTarget;

    unsafe impl NSObjectProtocol for AnimationTimerTarget {}

    impl AnimationTimerTarget {
        #[unsafe(method(tick:))]
        fn tick(&self, _: &NSTimer) {
            if self.ivars().animation.is_active() {
                self.ivars().tree_context.begin_batch();
                self.ivars().animation.tick();
                self.ivars().tree_context.end_batch();
            }
        }
    }
);

impl AnimationTimerTarget {
    fn new(mtm: MainThreadMarker, state: AnimationTimerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

fn apply_title_bar_mode(window: &NSWindow, mode: TitleBarMode) {
    let mut style_mask = window.styleMask();

    match mode {
        TitleBarMode::System => {
            style_mask.insert(NSWindowStyleMask::Titled);
            style_mask.remove(NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Visible);
            window.setTitlebarAppearsTransparent(false);
        }
        TitleBarMode::Hidden => {
            style_mask.remove(NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(false);
        }
        TitleBarMode::Overlay => {
            style_mask.insert(NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
        }
    }

    window.setStyleMask(style_mask);
}

struct WindowState {
    tree_context: Rc<TreeContext>,
    on_resize: PropValue<Option<Shared<dyn Fn(dpi::Size)>>>,
    on_close_requested: PropValue<Option<Shared<dyn Fn()>>>,
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
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _: &NSWindow) -> bool {
            // The callback may synchronously unmount the Window and release the
            // delegate retained by its lifecycle closure.
            let _delegate = unsafe {
                Retained::retain(std::ptr::from_ref(self).cast_mut())
                    .expect("window delegate must remain valid during close handling")
            };
            if let Some(on_close_requested) = self.ivars().on_close_requested.get() {
                on_close_requested();
            }
            false
        }

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
