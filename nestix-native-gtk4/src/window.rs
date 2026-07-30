use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gtk4::{glib, prelude::*};
use nestix::{
    Element, Layout, Readonly, callback, closure, component, components::ContextProvider, computed,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, AnimationRuntime, Length, StyleContext, StyleScope, TitleBarMode, TreeContext,
    WindowProps, WithAuto as NativeLengthWithAuto,
    dpi::{LogicalSize, Size as DpiSize},
    matched_style, style_length_with_auto,
};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{
    allocation_bin::AllocationBin,
    contexts::{LayoutRefreshContext, ParentContext},
};

#[derive(Clone)]
pub struct WindowContext {
    pub window: gtk4::Window,
    pub scale_factor: Readonly<f64>,
    pub animation: Rc<AnimationRuntime>,
    pub(crate) radio_buttons: Rc<RefCell<Vec<crate::radio_button::RegisteredRadioButton>>>,
    pub(crate) menu_bar_container: gtk4::Box,
    pub(crate) menu_bar: Rc<RefCell<Option<gtk4::PopoverMenuBar>>>,
    pub(crate) correct_content_size: Rc<Cell<bool>>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__gtk4_Window"];

    let tree_context = Rc::new(TreeContext::new());
    let layout_refresh = LayoutRefreshContext::new(tree_context.clone());
    let animation = Rc::new(AnimationRuntime::new());
    let (scale_factor, set_scale_factor) = create_state(1.0);
    let window = gtk4::Window::new();
    let radio_buttons = Rc::new(RefCell::new(Vec::new()));
    let menu_bar = Rc::new(RefCell::new(None::<gtk4::PopoverMenuBar>));
    let window_content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let overlay = gtk4::Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    let content = AllocationBin::new();
    let header_bar = gtk4::HeaderBar::new();
    let header_title = gtk4::Label::new(None);
    header_bar.set_title_widget(Some(&header_title));
    header_bar.set_show_title_buttons(true);
    header_bar.set_valign(gtk4::Align::Start);
    overlay.add_overlay(&header_bar);
    overlay.set_child(Some(&content));
    window_content.append(&overlay);
    window.set_child(Some(&window_content));
    let unmounting = Rc::new(Cell::new(false));
    let handling_close_request = Rc::new(Cell::new(false));
    set_scale_factor.set(window.scale_factor() as f64);
    element.provide_handle(window.clone());

    window.connect_scale_factor_notify(closure!(
        [scale_factor] | window | {
            set_scale_factor.set(window.scale_factor() as f64);
        }
    ));
    window.connect_close_request(closure!(
        [
            unmounting,
            handling_close_request,
            props.desktop.on_close_requested
        ] | _
            | {
                handle_close_request(
                    &unmounting,
                    &handling_close_request,
                    on_close_requested.get(),
                )
            }
    ));
    element.on_unmount(closure!(
        [window, unmounting, handling_close_request] || {
            unmounting.set(true);
            if !handling_close_request.get() {
                window.close();
            }
        }
    ));

    scoped_effect!(
        [window, header_title, props.title] || {
            let title = title.get();
            window.set_title(Some(&title));
            header_title.set_text(&title);
        }
    );
    let requested_content_size = Rc::new(Cell::new((-1, -1)));
    let decoration_size = Rc::new(Cell::new((0, 0)));
    let correct_content_size = Rc::new(Cell::new(false));
    let native_size_override = Rc::new(Cell::new(false));
    let style_props = matched_style(
        element.context::<StyleContext>(),
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
    let animated_size = Rc::new(AnimatedStyle::new(animation.clone(), target_size.get()));
    let presented_size = animated_size.value();
    scoped_effect!(
        [
            animated_size,
            target_size,
            scale_factor,
            native_size_override
        ] || {
            native_size_override.set(false);
            animated_size.set_target(target_size.get(), scale_factor.get());
        }
    );
    scoped_effect!(
        [
            window,
            requested_content_size,
            decoration_size,
            presented_size,
            scale_factor
        ] || {
            let style = presented_size.get().unwrap_or_default();
            let fallback = requested_content_size.get();
            let size = (
                logical_length(style.width, fallback.0, scale_factor.get()),
                logical_length(style.height, fallback.1, scale_factor.get()),
            );
            requested_content_size.set(size);
            let decoration = decoration_size.get();
            window.set_default_size(size.0 + decoration.0, size.1 + decoration.1);
        }
    );
    scoped_effect!(
        [window, props.visible] || {
            if visible.get() {
                window.present();
            } else {
                window.set_visible(false);
            }
        }
    );
    scoped_effect!(
        [window, props.desktop.resizable] || {
            window.set_resizable(resizable.get());
        }
    );
    scoped_effect!(
        [
            window,
            header_bar,
            decoration_size,
            correct_content_size,
            menu_bar,
            props.desktop.title_bar_mode
        ] || {
            let mode = title_bar_mode.get();
            apply_title_bar_mode(&window, &header_bar, mode);
            decoration_size.set((0, 0));
            correct_content_size.set(mode == TitleBarMode::System || menu_bar.borrow().is_some());
        }
    );

    let last_content_width = Rc::new(Cell::new(-1));
    let last_content_height = Rc::new(Cell::new(-1));
    window.add_tick_callback(closure!(
        [
            tree_context,
            layout_refresh,
            animation,
            props.on_resize,
            content,
            last_content_width,
            last_content_height,
            requested_content_size,
            decoration_size,
            correct_content_size,
            native_size_override,
            animated_size,
            presented_size
        ] | window,
        _ | {
            let width = window.width();
            let height = window.height();
            let content_width = content.width();
            let content_height = content.height();
            let correcting_content_size = correct_content_size.get();
            if correcting_content_size && content_width > 0 && content_height > 0 {
                let requested_size = requested_content_size.get();
                // GTK includes client-side system decorations in the window
                // allocation. Preserve the requested content size by adding the
                // measured decoration size to the default window size. With
                // server-side decorations this difference is zero.
                let decoration = (width - content_width, height - content_height);
                decoration_size.set(decoration);
                window.set_default_size(
                    requested_size.0 + decoration.0,
                    requested_size.1 + decoration.1,
                );
                correct_content_size.set(false);
            }
            let requested_size = requested_content_size.get();
            let native_size = (content_width, content_height);
            let differs_from_request = (native_size.0 - requested_size.0).abs() > 1
                || (native_size.1 - requested_size.1).abs() > 1;
            if content_width > 0
                && content_height > 0
                && !correcting_content_size
                && (native_size_override.get() || animated_size.is_active() && differs_from_request)
            {
                native_size_override.set(true);
                let mut presentation = presented_size.get().unwrap_or_default();
                presentation.width = Some(NativeLengthWithAuto::from(content_width));
                presentation.height = Some(NativeLengthWithAuto::from(content_height));
                animated_size.interrupt(Some(presentation));
            }
            if animation.is_active() {
                animation.tick();
                layout_refresh.flush_queued_refresh();
            }
            if content_width != last_content_width.get()
                || content_height != last_content_height.get()
            {
                last_content_width.set(content_width);
                last_content_height.set(content_height);
                if let Some(root_node) = tree_context.root_node() {
                    tree_context.update_style(root_node, |prev| Style {
                        size: Size {
                            width: taffy::Dimension::from_length(content_width.max(0) as f32),
                            height: taffy::Dimension::from_length(content_height.max(0) as f32),
                        },
                        ..prev
                    });
                    layout_refresh.queue_refresh();
                }
                if let Some(on_resize) = on_resize.get() {
                    on_resize(DpiSize::Logical(LogicalSize::new(
                        content_width as f64,
                        content_height as f64,
                    )));
                }
            }
            glib::ControlFlow::Continue
        }
    ));

    element.after_mount(closure!(
        [window, layout_refresh, props.visible] || {
            layout_refresh.flush_queued_refresh();
            if visible.get() {
                window.present();
            }
        }
    ));
    let window_context = Rc::new(WindowContext {
        window: window.clone(),
        scale_factor: scale_factor.into_readonly(),
        animation,
        radio_buttons,
        menu_bar_container: window_content,
        menu_bar,
        correct_content_size,
    });

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                ContextProvider<LayoutRefreshContext>(layout_refresh.clone()) {
                    StyleScope(
                        .class = props.class.clone(),
                        .default_classes = DEFAULT_CLASSES,
                        .effective_style = target_size,
                    ) {
                        ContextProvider<nestix_native_core::NativeVisualMount>(
                            nestix_native_core::NativeVisualMount::allowed(crate::GTK4_BACKEND_ID),
                        ) {
                            ContextProvider<ParentContext>(
                                ParentContext {
                                    fixed: None,
                                    add_child: Some(callback!([content, tree_context, layout_refresh] |widget: &gtk4::Widget,
                                    child_node: Option<NodeId> | {
                                        content.set_child(Some(widget));
                                        tree_context.set_root_node(child_node);
                                        if let Some(child_node) = child_node {
                                            let width = content.width().max(0) as f32;
                                            let height = content.height().max(0) as f32;
                                            tree_context.update_style(child_node, |prev| Style {
                                                size: Size {
                                                    width: taffy::Dimension::from_length(width),
                                                    height: taffy::Dimension::from_length(height),
                                                },
                                                ..prev
                                            });
                                            layout_refresh.queue_refresh();
                                        }
                                    })),
                                    insert_child: None,
                                    remove_child: Some(callback!([content, tree_context] |_: &gtk4::Widget,
                                    _: Option<NodeId> | {
                                        content.set_child(gtk4::Widget::NONE);
                                        tree_context.set_root_node(None);
                                    })),
                                    parent_node: None
                                },
                            ) {
                                $(props.children.clone().map(|child| Layout::from(child.clone())))
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_close_request(
    unmounting: &Cell<bool>,
    handling_close_request: &Cell<bool>,
    on_close_requested: Option<nestix::Shared<dyn Fn()>>,
) -> glib::Propagation {
    if unmounting.get() {
        return glib::Propagation::Proceed;
    }
    handling_close_request.set(true);
    if let Some(on_close_requested) = on_close_requested {
        on_close_requested();
    }
    handling_close_request.set(false);
    if unmounting.get() {
        // The callback synchronously unmounted this component. Let the
        // original GTK close request finish closing the window.
        glib::Propagation::Proceed
    } else {
        glib::Propagation::Stop
    }
}

fn logical_length(
    value: Option<NativeLengthWithAuto<Length>>,
    fallback: i32,
    scale_factor: f64,
) -> i32 {
    match value {
        Some(NativeLengthWithAuto::Value(value)) => {
            value.to_logical::<f64>(scale_factor).0.round() as i32
        }
        Some(NativeLengthWithAuto::Auto) | None => fallback,
    }
}

fn apply_title_bar_mode(
    window: &gtk4::Window,
    overlay_header: &gtk4::HeaderBar,
    mode: TitleBarMode,
) {
    match mode {
        TitleBarMode::System => {
            overlay_header.set_visible(false);
            window.set_decorated(true);
        }
        TitleBarMode::Hidden => {
            overlay_header.set_visible(false);
            window.set_decorated(false);
        }
        TitleBarMode::Overlay => {
            window.set_decorated(false);
            overlay_header.set_visible(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nestix::Shared;

    #[test]
    fn close_proceeds_when_callback_synchronously_unmounts() {
        let unmounting = Rc::new(Cell::new(false));
        let handling_close_request = Rc::new(Cell::new(false));
        let callback = Shared::from(Rc::new({
            let unmounting = unmounting.clone();
            let handling_close_request = handling_close_request.clone();
            move || {
                assert!(handling_close_request.get());
                unmounting.set(true);
            }
        }) as Rc<dyn Fn()>);

        assert_eq!(
            handle_close_request(&unmounting, &handling_close_request, Some(callback)),
            glib::Propagation::Proceed
        );
        assert!(!handling_close_request.get());
    }

    #[test]
    fn close_stops_when_component_remains_mounted() {
        assert_eq!(
            handle_close_request(&Cell::new(false), &Cell::new(false), None),
            glib::Propagation::Stop
        );
    }
}
