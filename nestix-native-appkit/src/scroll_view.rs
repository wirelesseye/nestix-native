use std::rc::Rc;

use nestix::Layout;
use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};
use nestix_native_core::{
    Dimension, ScrollViewProps, StyleContext, StyleScope, TreeContext, matched_style,
    resolved_view_style, style_align_self, style_dimension, style_flex_basis, style_flex_grow,
    style_flex_shrink, style_margin,
};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSScrollView, NSView};
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize};
use taffy::style_helpers::FromLength;
use taffy::{NodeId, Size, Style};

use crate::{
    WindowContext,
    contexts::{ParentContext, native_child_index},
};

#[component]
pub fn ScrollView(props: &ScrollViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__ScrollView", "__appkit_ScrollView"];

    let window = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let styles = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(styles.clone(), &props.view);
    let mtm = MainThreadMarker::new().unwrap();
    let scroll = NSScrollView::new(mtm);
    scroll.setDrawsBackground(false);
    element.provide_handle(scroll.as_ref() as *const NSObject);
    let node = tree_context.create_node(false);

    element.on_place(closure!(
        [element, scroll, parent] | placement | {
            if placement.index.is_some()
                && let Some(insert) = &parent.insert_child
            {
                insert(&scroll, Some(node), native_child_index(&element));
            } else if let Some(add) = &parent.add_child {
                add(&scroll, Some(node));
            }
        }
    ));

    scoped_effect!(
        element,
        [scroll, props.scroll_x, props.scroll_y] || {
            scroll.setHasHorizontalScroller(scroll_x.get());
            scroll.setHasVerticalScroller(scroll_y.get());
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            styles,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            props.view.align_self,
            window.scale_factor
        ] || {
            let style = styles.get();
            tree_context.update_style(node, |prev| Style {
                flex_grow: style_flex_grow(style.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style.as_ref(), flex_shrink.get()),
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window.scale_factor,
            tree_context,
            styles,
            props.view.width,
            props.view.height,
            props.view.left,
            props.view.top,
            props.view.margin()
        ] || {
            let scale = scale_factor.get();
            let style = styles.get();
            let width = style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width);
            let height =
                style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| s.height);
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node, |prev| Style {
                flex_direction: taffy::FlexDirection::Column,
                size: Size {
                    width: width.to_taffy(scale),
                    height: height.to_taffy(scale),
                },
                inset: inset_to_taffy(left, top, scale),
                margin: margin_to_taffy(style_margin(style.as_ref(), margin.get()), scale),
                ..prev
            });
            tree_context.refresh();
        }
    );

    let subtree_context = Rc::new(TreeContext::new());
    let subtree_root = subtree_context.create_node(false);
    subtree_context.set_root_node(Some(subtree_root));

    scoped_effect!(
        element,
        [tree_context, subtree_context, parent.parent_node, scroll] || {
            if parent_node.is_some()
                && let Some(value) = tree_context.layout(node)
            {
                scroll.setFrame(NSRect::new(
                    NSPoint::new(value.location.x.into(), value.location.y.into()),
                    NSSize::new(value.size.width.into(), value.size.height.into()),
                ));

                let content_size = scroll.contentSize();
                subtree_context.update_style(subtree_root, |prev| Style {
                    min_size: Size {
                        width: taffy::Dimension::from_length(content_size.width as f32),
                        height: taffy::Dimension::from_length(content_size.height as f32),
                    },
                    ..prev
                });
                subtree_context.refresh();
            }
        }
    );

    element.on_unmount(closure!([scroll] || scroll.removeFromSuperview()));

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style
        ) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<ParentContext>(ParentContext {
                    add_child: Some(callback!([scroll, subtree_context] |object: &NSObject, child_node: Option<NodeId>| {
                        let view = object.downcast_ref::<NSView>().unwrap();
                        scroll.setDocumentView(Some(view));
                        if let Some(child_node) = child_node {
                            subtree_context.add_child(subtree_root, child_node);
                            subtree_context.refresh();
                        }
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([scroll] |_: &NSObject, child_node: Option<NodeId>| {
                        scroll.setDocumentView(None);
                        if let Some(child_node) = child_node{
                            subtree_context.remove_child(subtree_root, child_node);
                        }
                    })),
                    parent_node: Some(subtree_root),
                }) {
                    $(props.children.clone().map(|element| Layout::from(element.clone())))
                }
            }
        }
    }
}
