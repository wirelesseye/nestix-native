use std::{cell::RefCell, rc::Rc};

use nestix::{
    Element, Layout, PropValue, Readonly, Shared, State, StateSetter, callback, closure, component,
    components::ContextProvider, computed, create_state, layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, AnimationRuntime, Length, Material, MaterialSource, StyleContext, StyleScope,
    TitlebarMode, TreeContext, WindowProps, WithAuto as NativeLengthWithAuto,
    dpi::{self, LogicalSize},
    matched_style, style_length_with_auto,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject, sel,
};
use objc2_app_kit::{
    NSColor, NSMenu, NSToolbar, NSView, NSVisualEffectBlendingMode, NSVisualEffectView, NSWindow,
    NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSSize, NSString, NSTimer};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};

use crate::{
    contexts::ParentContext, material::visual_effect_view, root::RootContext,
    sidebar::MountedSidebar,
};

pub struct WindowContext {
    pub ns_window: Retained<NSWindow>,
    pub scale_factor: Readonly<f64>,
    pub animation: Rc<AnimationRuntime>,
    pub(crate) main_content_host: Retained<ContentHost>,
    pub(crate) sidebar: RefCell<Option<MountedSidebar>>,
    pub(crate) menu: State<Option<Retained<NSMenu>>>,
    pub(crate) set_menu: StateSetter<Option<Retained<NSMenu>>>,
    pub(crate) toolbar: State<Option<Retained<NSToolbar>>>,
    pub(crate) set_toolbar: StateSetter<Option<Retained<NSToolbar>>>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__appkit_Window"];

    let mtm = MainThreadMarker::new().unwrap();
    let (scale_factor, set_scale_factor) = create_state(1.0);
    let (menu, set_menu) = create_state(None::<Retained<NSMenu>>);
    let (toolbar, set_toolbar) = create_state(None::<Retained<NSToolbar>>);
    let root_context = element.context::<RootContext>().unwrap();
    let style_context = element.context::<StyleContext>();

    let ns_window = unsafe { NSWindow::new(mtm) };
    let tree_context = Rc::new(TreeContext::new());
    let main_content_host = ContentHost::new(mtm, tree_context.clone());
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
        main_content_host: main_content_host.clone(),
        sidebar: RefCell::new(None),
        menu: menu.clone(),
        set_menu: set_menu.clone(),
        toolbar,
        set_toolbar,
    });

    let window_delegate = WindowDelegate::new(
        mtm,
        WindowState {
            main_content_host: main_content_host.clone(),
            on_resize: props.on_resize.clone(),
            on_close_requested: props.desktop.on_close_requested.clone(),
            menu,
            active_window_menu: root_context.active_window_menu.clone(),
            set_active_window_menu: root_context.set_active_window_menu.clone(),
        },
    );
    let style_mask = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;
    ns_window.setStyleMask(style_mask);
    apply_titlebar_mode(&ns_window, props.desktop.titlebar_mode.get());
    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*window_delegate)));
    ns_window.setContentView(Some(&main_content_host));

    let original_opaque = ns_window.isOpaque();
    let original_background_color = ns_window.backgroundColor();

    scoped_effect!(
        [
            ns_window,
            main_content_host,
            original_background_color,
            props.desktop.material,
            props.desktop.material_source
        ] || {
            let material = material.get();
            let has_appkit_material =
                material.is_some_and(|material| material.macos_material().is_some());
            main_content_host.set_material(material, material_source.get());
            if has_appkit_material {
                ns_window.setOpaque(false);
                ns_window.setBackgroundColor(Some(&NSColor::clearColor()));
            } else {
                ns_window.setOpaque(original_opaque);
                ns_window.setBackgroundColor(Some(&original_background_color));
            }
        }
    );

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

    set_scale_factor.set(ns_window.backingScaleFactor());

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
        [ns_window, props.desktop.resizable] || {
            let mut style_mask = ns_window.styleMask();
            if resizable.get() {
                style_mask.insert(NSWindowStyleMask::Resizable);
            } else {
                style_mask.remove(NSWindowStyleMask::Resizable);
            }
            ns_window.setStyleMask(style_mask);
        }
    );

    scoped_effect!(
        [ns_window, props.desktop.titlebar_mode] || {
            apply_titlebar_mode(&ns_window, titlebar_mode.get());
        }
    );

    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_size = computed!(
        [style_props, props.desktop.width, props.desktop.height] || {
            let mut style = style_props.get().unwrap_or_default();
            style.width = Some(style_length_with_auto(
                Some(&style),
                width.get().into(),
                NativeLengthWithAuto::from(800),
                |style| style.width,
            ));
            style.height = Some(style_length_with_auto(
                Some(&style),
                height.get().into(),
                NativeLengthWithAuto::from(600),
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
                    ContextProvider<nestix_native_core::NativeVisualMount>(
                        nestix_native_core::NativeVisualMount::allowed(crate::APPKIT_BACKEND_ID),
                    ) {
                        ContextProvider<ParentContext>(
                            ParentContext {
                                add_child: Some(callback!([main_content_host] |object: &NSObject,
                                child_node: Option<NodeId> | {
                                    let view = object.downcast_ref::<NSView>().unwrap();
                                    main_content_host.set_child(view, child_node);
                                })),
                                insert_child: None,
                                remove_child: Some(callback!([main_content_host] |object: &NSObject,
                                _: Option<NodeId> | {
                                    let view = object.downcast_ref::<NSView>().unwrap();
                                    main_content_host.remove_child(view);
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
}

pub(crate) struct ContentHostState {
    tree_context: Rc<TreeContext>,
    child: RefCell<Option<Retained<NSView>>>,
    material: RefCell<Option<Retained<NSVisualEffectView>>>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ContentHostState]
    pub(crate) struct ContentHost;

    unsafe impl NSObjectProtocol for ContentHost {}

    impl ContentHost {
        #[unsafe(method(layout))]
        fn layout(&self) {
            unsafe {
                let _: () = msg_send![super(self), layout];
            }
            self.resize_tree();
        }
    }
);

impl ContentHost {
    pub(crate) fn new(mtm: MainThreadMarker, tree_context: Rc<TreeContext>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ContentHostState {
            tree_context,
            child: RefCell::new(None),
            material: RefCell::new(None),
        });
        unsafe { msg_send![super(this), init] }
    }

    pub(crate) fn set_child(&self, child: &NSView, child_node: Option<NodeId>) {
        let already_mounted = self
            .ivars()
            .child
            .borrow()
            .as_ref()
            .is_some_and(|current| std::ptr::eq::<NSView>(current.as_ref(), child));
        if !already_mounted {
            if let Some(previous) = self.ivars().child.borrow_mut().take() {
                previous.removeFromSuperview();
            }
            child.removeFromSuperview();
            self.addSubview(child);
            self.ivars().child.replace(Some(child.retain()));
        }
        self.ivars().tree_context.set_root_node(child_node);
        self.resize_tree();
    }

    pub(crate) fn set_material(&self, material: Option<Material>, source: MaterialSource) {
        if let Some(previous) = self.ivars().material.borrow_mut().take() {
            previous.removeFromSuperview();
        }

        let Some(material) = material else { return };
        let Some(effect) = visual_effect_view(
            MainThreadMarker::new().unwrap(),
            material,
            source,
            NSVisualEffectBlendingMode::BehindWindow,
        ) else {
            return;
        };
        effect.setFrame(self.bounds());
        self.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, None);
        self.ivars().material.replace(Some(effect));
    }

    pub(crate) fn remove_child(&self, child: &NSView) {
        let owns_child = self
            .ivars()
            .child
            .borrow()
            .as_ref()
            .is_some_and(|current| std::ptr::eq::<NSView>(current.as_ref(), child));
        if !owns_child {
            return;
        }

        self.ivars().child.borrow_mut().take();
        child.removeFromSuperview();
        self.ivars().tree_context.set_root_node(None);
    }

    pub(crate) fn resize_tree(&self) {
        let tree_context = &self.ivars().tree_context;
        let bounds = self.bounds();
        if let Some(material) = self.ivars().material.borrow().as_ref() {
            material.setFrame(bounds);
        }
        if let Some(child) = self.ivars().child.borrow().as_ref() {
            // A Taffy root has no native parent node, so its component does not
            // assign its own frame. NSWindow and NSTabView normally size such
            // roots for us; this intermediate host must do the same.
            child.setFrame(bounds);
        }
        let Some(root_node) = tree_context.root_node() else {
            return;
        };
        let size = bounds.size;
        tree_context.update_style(root_node, |prev| Style {
            size: Size {
                width: Dimension::from_length(size.width.max(0.0) as f32),
                height: Dimension::from_length(size.height.max(0.0) as f32),
            },
            ..prev
        });
        tree_context.refresh();
    }
}

fn logical_length(
    value: Option<NativeLengthWithAuto<Length>>,
    fallback: f64,
    scale_factor: f64,
) -> f64 {
    match value {
        Some(NativeLengthWithAuto::Value(value)) => value.to_logical::<f64>(scale_factor).0,
        Some(NativeLengthWithAuto::Auto) | None => fallback,
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

fn apply_titlebar_mode(window: &NSWindow, mode: TitlebarMode) {
    let mut style_mask = window.styleMask();

    match mode {
        TitlebarMode::System => {
            style_mask.insert(NSWindowStyleMask::Titled);
            style_mask.remove(NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Visible);
            window.setTitlebarAppearsTransparent(false);
        }
        TitlebarMode::Hidden => {
            style_mask.remove(NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(false);
        }
        TitlebarMode::Overlay => {
            style_mask.insert(NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
        }
    }

    window.setStyleMask(style_mask);
}

struct WindowState {
    main_content_host: Retained<ContentHost>,
    on_resize: PropValue<Option<Shared<dyn Fn(dpi::Size)>>>,
    on_close_requested: PropValue<Option<Shared<dyn Fn()>>>,
    menu: State<Option<Retained<NSMenu>>>,
    active_window_menu: State<Option<Retained<NSMenu>>>,
    set_active_window_menu: StateSetter<Option<Retained<NSMenu>>>,
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
            self.ivars()
                .set_active_window_menu
                .set(self.ivars().menu.get());
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _: &NSNotification) {
            let menu = self.ivars().menu.get();
            let active = self.ivars().active_window_menu.get();
            if same_menu(&active, &menu) {
                self.ivars().set_active_window_menu.set(None);
            }
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, notification: &NSNotification) {
            let window = notification
                .object()
                .unwrap()
                .downcast::<NSWindow>()
                .unwrap();
            // Replacing the window's content view controller can synchronously
            // deliver a resize notification while AppKit has no content view
            // installed. Derive the content size from the window frame so the
            // callback remains valid throughout that transition.
            let size = window.contentRectForFrameRect(window.frame()).size;

            self.ivars().main_content_host.resize_tree();

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
