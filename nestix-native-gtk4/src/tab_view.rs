use std::{cell::Cell, rc::Rc};

use gtk4::prelude::*;
use nestix::{
    Element, Layout, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    StyleContext, StyleScope, TabViewItemProps, TabViewProps, TreeContext, matched_style,
    resolved_view_style,
};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{
    allocation_bin::AllocationBin, contexts::ParentContext,
    layout::mount_leaf_with_stretchable_width,
};

#[derive(Clone)]
struct TabViewContext {
    notebook: gtk4::Notebook,
    content_revision: State<usize>,
}

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabView", "__gtk4_TabView"];

    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(style_props.clone(), &props.view);
    let notebook = gtk4::Notebook::new();
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    let content_revision = create_state(0usize);
    let node_id = mount_leaf_with_stretchable_width(
        element,
        notebook.upcast_ref(),
        style_props,
        &props.view,
        content_revision.clone().into_readonly(),
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style
        ) {
            ContextProvider<TabViewContext>(TabViewContext {
                notebook: notebook.clone(),
                content_revision: content_revision.clone(),
            }) {
                ContextProvider<ParentContext>(ParentContext {
                    fixed: None,
                    add_child: Some(callback!([notebook, content_revision] |child: &gtk4::Widget, _: Option<NodeId>| {
                        remove_page(&notebook, child);
                        notebook.append_page(child, gtk4::Widget::NONE);
                        content_revision.mutate(|revision| *revision += 1);
                    })),
                    insert_child: Some(callback!([notebook, content_revision] |child: &gtk4::Widget, _: Option<NodeId>, predecessor: Option<gtk4::Widget>| {
                        remove_page(&notebook, child);
                        let position = predecessor
                            .as_ref()
                            .and_then(|predecessor| notebook.page_num(predecessor))
                            .map_or(0, |position| position + 1);
                        notebook.insert_page(child, gtk4::Widget::NONE, Some(position));
                        content_revision.mutate(|revision| *revision += 1);
                    })),
                    remove_child: Some(callback!([notebook, content_revision] |child: &gtk4::Widget, _: Option<NodeId>| {
                        remove_page(&notebook, child);
                        content_revision.mutate(|revision| *revision += 1);
                    })),
                    parent_node: Some(node_id),
                }) {
                    $(props.children.clone())
                }
            }
        }
    }
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabViewItem", "__gtk4_TabViewItem"];

    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_view_context = element.context::<TabViewContext>().unwrap();
    let page = AllocationBin::new();
    let page_widget: gtk4::Widget = page.clone().upcast();
    let label = gtk4::Label::new(Some(&props.title.get()));
    let subtree_context = Rc::new(TreeContext::new());
    element.provide_handle(page_widget.clone());

    element.on_place(closure!(
        [page_widget, label, parent_context, tab_view_context] | placement | {
            parent_context.place_child(&page_widget, None, placement);
            tab_view_context
                .notebook
                .set_tab_label(&page_widget, Some(&label));
        }
    ));
    element.on_unmount(closure!(
        [page_widget, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&page_widget, None);
            }
        }
    ));

    scoped_effect!(
        element,
        [tab_view_context, label, props.title] || {
            label.set_text(&title.get());
            tab_view_context
                .content_revision
                .mutate(|revision| *revision += 1);
        }
    );

    let last_width = Rc::new(Cell::new(-1));
    let last_height = Rc::new(Cell::new(-1));
    page.add_tick_callback(closure!(
        [subtree_context, last_width, last_height] | page,
        _ | {
            let width = page.width();
            let height = page.height();
            if width != last_width.get() || height != last_height.get() {
                last_width.set(width);
                last_height.set(height);
                resize_subtree(&subtree_context, width, height);
            }
            gtk4::glib::ControlFlow::Continue
        }
    ));

    layout! {
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<ParentContext>(ParentContext {
                    fixed: None,
                    add_child: Some(callback!([page, subtree_context] |child: &gtk4::Widget, child_node: Option<NodeId>| {
                        page.set_child(Some(child));
                        subtree_context.set_root_node(child_node);
                        resize_subtree(&subtree_context, page.width(), page.height());
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([page, subtree_context] |child: &gtk4::Widget, _: Option<NodeId>| {
                        if page.child().as_ref() == Some(child) {
                            page.set_child(gtk4::Widget::NONE);
                            subtree_context.set_root_node(None);
                        }
                    })),
                    parent_node: None,
                }) {
                    $(props.children.clone().map(|child| Layout::from(child.clone())))
                }
            }
        }
    }
}

fn remove_page(notebook: &gtk4::Notebook, child: &gtk4::Widget) {
    if let Some(position) = notebook.page_num(child) {
        notebook.remove_page(Some(position));
    }
}

fn resize_subtree(tree_context: &TreeContext, width: i32, height: i32) {
    let Some(root_node) = tree_context.root_node() else {
        return;
    };
    tree_context.update_style(root_node, |prev| Style {
        size: Size {
            width: taffy::Dimension::from_length(width.max(0) as f32),
            height: taffy::Dimension::from_length(height.max(0) as f32),
        },
        ..prev
    });
    tree_context.refresh();
}
