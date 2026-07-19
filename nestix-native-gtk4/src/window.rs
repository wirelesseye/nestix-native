use std::{cell::Cell, rc::Rc};

use gtk4::{glib, prelude::*};
use nestix::{
    Element, Layout, Readonly, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    StyleScope, TitleBarMode, TreeContext, WindowProps,
    dpi::{LogicalSize, Size as DpiSize},
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
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Window", "__gtk4_Window"];

    let tree_context = Rc::new(TreeContext::new());
    let layout_refresh = LayoutRefreshContext::new(tree_context.clone());
    let scale_factor = create_state(1.0);
    let window = gtk4::Window::new();
    let overlay = gtk4::Overlay::new();
    let content = AllocationBin::new();
    let header_bar = gtk4::HeaderBar::new();
    let header_title = gtk4::Label::new(None);
    header_bar.set_title_widget(Some(&header_title));
    header_bar.set_show_title_buttons(true);
    header_bar.set_valign(gtk4::Align::Start);
    overlay.add_overlay(&header_bar);
    overlay.set_child(Some(&content));
    window.set_child(Some(&overlay));
    let unmounting = Rc::new(Cell::new(false));
    scale_factor.set(window.scale_factor() as f64);
    element.provide_handle(window.clone());

    window.connect_scale_factor_notify(closure!(
        [scale_factor] | window | {
            scale_factor.set(window.scale_factor() as f64);
        }
    ));
    window.connect_close_request(closure!(
        [unmounting, props.on_close_requested] | _ | {
            if unmounting.get() {
                glib::Propagation::Proceed
            } else {
                if let Some(on_close_requested) = on_close_requested.get() {
                    on_close_requested();
                }
                glib::Propagation::Stop
            }
        }
    ));
    element.on_unmount(closure!(
        [window, unmounting] || {
            unmounting.set(true);
            window.close();
        }
    ));

    scoped_effect!(
        element,
        [window, header_title, props.title] || {
            let title = title.get();
            window.set_title(Some(&title));
            header_title.set_text(&title);
        }
    );
    scoped_effect!(
        element,
        [window, props.width, props.height] || {
            window.set_default_size(width.get().round() as i32, height.get().round() as i32);
        }
    );
    scoped_effect!(
        element,
        [window, header_bar, props.title_bar_mode] || {
            apply_title_bar_mode(&window, &header_bar, title_bar_mode.get());
        }
    );

    let last_width = Rc::new(Cell::new(-1));
    let last_height = Rc::new(Cell::new(-1));
    let last_content_width = Rc::new(Cell::new(-1));
    let last_content_height = Rc::new(Cell::new(-1));
    window.add_tick_callback(closure!(
        [
            tree_context,
            layout_refresh,
            props.on_resize,
            content,
            last_width,
            last_height,
            last_content_width,
            last_content_height
        ] | window,
        _ | {
            let width = window.width();
            let height = window.height();
            let content_width = content.width();
            let content_height = content.height();
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
            }
            if width != last_width.get() || height != last_height.get() {
                last_width.set(width);
                last_height.set(height);
                if let Some(on_resize) = on_resize.get() {
                    on_resize(DpiSize::Logical(LogicalSize::new(
                        width as f64,
                        height as f64,
                    )));
                }
            }
            glib::ControlFlow::Continue
        }
    ));

    element.after_mount(closure!(
        [window, layout_refresh] || {
            layout_refresh.flush_queued_refresh();
            window.present();
        }
    ));
    let window_context = Rc::new(WindowContext {
        window: window.clone(),
        scale_factor: scale_factor.into_readonly(),
    });

    layout! {
        ContextProvider<WindowContext>(window_context) {
            ContextProvider<TreeContext>(tree_context.clone()) {
                ContextProvider<LayoutRefreshContext>(layout_refresh.clone()) {
                    StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
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
