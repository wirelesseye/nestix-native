use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gtk4::prelude::*;
use nestix::{
    Element, Layout, callback, closure, component, components::ContextProvider, create_state,
    layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, ChildOrder, ScrollViewProps, StyleContext, StyleScope, TreeContext,
    matched_style, resolved_view_style,
};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{LayoutRefreshContext, ParentContext},
    layout::mount_leaf_with_stretchable_width,
};

#[component]
pub fn ScrollView(props: &ScrollViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__ScrollView", "__gtk4_ScrollView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let matched_style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched_style_props, &props.view);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        effective_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, effective_style, window_context.scale_factor] || {
            animated_style.set_target(effective_style.get(), scale_factor.get());
        }
    );
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hexpand(true);
    scrolled.set_vexpand(true);
    let content = gtk4::Fixed::new();
    scrolled.set_child(Some(&content));
    let content_revision = create_state(0usize);
    mount_leaf_with_stretchable_width(
        element,
        scrolled.upcast_ref(),
        style_props.into_readonly(),
        &props.view,
        content_revision.clone().into_readonly(),
    );

    let last_scroll_policy = Rc::new(Cell::new(None));
    scoped_effect!(
        [
            scrolled,
            content_revision,
            last_scroll_policy,
            props.scroll_x,
            props.scroll_y
        ] || {
            let policy = (scroll_x.get(), scroll_y.get());
            scrolled.set_policy(scrollbar_policy(policy.0), scrollbar_policy(policy.1));
            if last_scroll_policy.replace(Some(policy)) != Some(policy) {
                content_revision.mutate(|revision| *revision += 1);
            }
        }
    );

    let subtree_context = Rc::new(TreeContext::new());
    let subtree_refresh = LayoutRefreshContext::new(subtree_context.clone());
    let subtree_root = subtree_context.create_node(false);
    subtree_context.set_root_node(Some(subtree_root));
    let child_order = Rc::new(RefCell::new(ChildOrder::<gtk4::Widget>::new()));

    let last_viewport_width = Rc::new(Cell::new(-1.0));
    let last_viewport_height = Rc::new(Cell::new(-1.0));
    scrolled.add_tick_callback(closure!(
        [
            scrolled,
            subtree_context,
            subtree_refresh,
            last_viewport_width,
            last_viewport_height
        ] | _,
        _ | {
            let viewport_width = scrolled.hadjustment().page_size().max(0.0);
            let viewport_height = scrolled.vadjustment().page_size().max(0.0);
            if viewport_width != last_viewport_width.get()
                || viewport_height != last_viewport_height.get()
            {
                last_viewport_width.set(viewport_width);
                last_viewport_height.set(viewport_height);
                subtree_context.update_style(subtree_root, |prev| Style {
                    min_size: Size {
                        width: taffy::Dimension::from_length(viewport_width as f32),
                        height: taffy::Dimension::from_length(viewport_height as f32),
                    },
                    ..prev
                });
                subtree_refresh.queue_refresh();
            }
            gtk4::glib::ControlFlow::Continue
        }
    ));

    let last_content_size = Rc::new(Cell::new((-1, -1)));
    scoped_effect!(
        [
            subtree_context,
            content,
            content_revision,
            last_content_size
        ] || {
            if let Some(layout) = subtree_context.layout(subtree_root) {
                let size = (
                    layout.size.width.round() as i32,
                    layout.size.height.round() as i32,
                );
                if last_content_size.replace(size) != size {
                    content.set_size_request(size.0, size.1);
                    content_revision.mutate(|revision| *revision += 1);
                }
            }
        }
    );
    element.after_mount(closure!(
        [subtree_refresh] || subtree_refresh.flush_queued_refresh()
    ));

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<LayoutRefreshContext>(subtree_refresh.clone()) {
                    ContextProvider<ParentContext>(
                        ParentContext {
                            fixed: Some(content.clone()),
                            add_child: Some(callback!([content, subtree_context, subtree_refresh, child_order] |child: &gtk4::Widget,
                            child_node: Option<NodeId> | {
                                if child.parent().is_none() {
                                    content.put(child, 0.0, 0.0);
                                }
                                let predecessor = child_order.borrow().last_key();
                                child_order.borrow_mut().place(
                                    child.clone(),
                                    child_node,
                                    predecessor,
                                );
                                let nodes = child_order.borrow().taffy_nodes();
                                subtree_context.set_children(subtree_root, &nodes);
                                subtree_refresh.queue_refresh();
                            })),
                            insert_child: Some(callback!([content, subtree_context, subtree_refresh, child_order] |child: &gtk4::Widget,
                            child_node: Option<NodeId>,
                            predecessor: Option<gtk4::Widget> | {
                                if child.parent().is_none() {
                                    content.put(child, 0.0, 0.0);
                                }
                                child_order.borrow_mut().place(
                                    child.clone(),
                                    child_node,
                                    predecessor,
                                );
                                let nodes = child_order.borrow().taffy_nodes();
                                subtree_context.set_children(subtree_root, &nodes);
                                subtree_refresh.queue_refresh();
                            })),
                            remove_child: Some(callback!([content, subtree_context, subtree_refresh, child_order] |child: &gtk4::Widget,
                            _: Option<NodeId> | {
                                if child.parent().as_ref() == Some(content.upcast_ref()) {
                                    content.remove(child);
                                }
                                child_order.borrow_mut().remove(child.clone());
                                let nodes = child_order.borrow().taffy_nodes();
                                subtree_context.set_children(subtree_root, &nodes);
                                subtree_refresh.queue_refresh();
                            })),
                            parent_node: Some(subtree_root)
                        },
                    ) {
                        $(props.children.clone().map(|child| Layout::from(child.clone())))
                    }
                }
            }
        }
    }
}

fn scrollbar_policy(enabled: bool) -> gtk4::PolicyType {
    if enabled {
        gtk4::PolicyType::Automatic
    } else {
        gtk4::PolicyType::Never
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_axes_never_show_a_scrollbar() {
        assert_eq!(scrollbar_policy(true), gtk4::PolicyType::Automatic);
        assert_eq!(scrollbar_policy(false), gtk4::PolicyType::Never);
    }
}
