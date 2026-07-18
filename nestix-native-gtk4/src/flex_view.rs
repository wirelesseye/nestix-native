use std::{cell::RefCell, rc::Rc};

use gtk4::prelude::*;
use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    ChildOrder, Dimension, FlexViewProps, StyleContext, StyleScope, TreeContext, matched_style,
    resolved_flex_view_style, style_align_items, style_align_self, style_dimension,
    style_flex_basis, style_flex_direction, style_flex_grow, style_flex_shrink, style_flex_wrap,
    style_gap, style_justify_content, style_margin, style_padding,
    utils::{gap_to_taffy, inset_to_taffy, margin_to_taffy, padding_to_taffy},
};
use taffy::{NodeId, Size, Style};

use crate::{WindowContext, contexts::ParentContext};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__gtk4_FlexView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(style_props.clone(), props);
    let fixed = gtk4::Fixed::new();
    fixed.set_hexpand(true);
    fixed.set_vexpand(true);
    let widget: gtk4::Widget = fixed.clone().upcast();
    let node_id = tree_context.create_node(false);
    let child_order = Rc::new(RefCell::new(ChildOrder::<gtk4::Widget>::new()));
    element.provide_handle(widget.clone());

    element.on_place(closure!(
        [widget, parent_context] | placement | {
            parent_context.place_child(&widget, Some(node_id), placement);
        }
    ));
    element.on_unmount(closure!(
        [widget, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&widget, Some(node_id));
            }
        }
    ));

    let css = gtk4::CssProvider::new();
    fixed
        .style_context()
        .add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    scoped_effect!(
        element,
        [css, style_props, props.bg_color] || {
            let style_props = style_props.get();
            let color = bg_color
                .get()
                .or_else(|| style_props.as_ref().and_then(|style| style.bg_color));
            let declaration = color
                .map(|color| {
                    let color = color.into_rgb();
                    format!(
                        "background-color: rgba({}, {}, {}, {:.3});",
                        color.red,
                        color.green,
                        color.blue,
                        color.alpha as f64 / 255.0,
                    )
                })
                .unwrap_or_default();
            css.load_from_data(&format!("fixed {{ {declaration} }}"));
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            props.view.width,
            props.view.height,
            props.view.left,
            props.view.top,
            props.view.margin(),
            props.view.align_self,
            props.container.padding(),
            props.gap,
            props.flex_direction,
            props.align_items,
            props.justify_content,
            props.flex_wrap,
            window_context.scale_factor
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            );
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
                style.top
            });
            let gap = gap_to_taffy(style_gap(style_props.as_ref(), gap.get()), scale_factor);
            tree_context.update_style(node_id, |prev| Style {
                size: if parent_node.is_some() {
                    Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
                    }
                } else {
                    prev.size
                },
                inset: inset_to_taffy(left, top, scale_factor),
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                padding: padding_to_taffy(
                    style_padding(style_props.as_ref(), padding.get()),
                    scale_factor,
                ),
                gap: Size {
                    width: gap,
                    height: gap,
                },
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                flex_direction: style_flex_direction(style_props.as_ref(), flex_direction.get())
                    .to_taffy(),
                align_items: style_align_items(style_props.as_ref(), align_items.get()).to_taffy(),
                justify_content: style_justify_content(style_props.as_ref(), justify_content.get())
                    .to_taffy(),
                flex_wrap: style_flex_wrap(style_props.as_ref(), flex_wrap.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            parent_context.parent_node,
            parent_context.fixed,
            widget
        ] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                widget.set_size_request(
                    layout.size.width.round() as i32,
                    layout.size.height.round() as i32,
                );
                if let Some(parent_fixed) = &fixed
                    && widget.parent().as_ref() == Some(parent_fixed.upcast_ref())
                {
                    parent_fixed.move_(&widget, layout.location.x as f64, layout.location.y as f64);
                }
            }
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style
        ) {
            ContextProvider<ParentContext>(ParentContext {
                fixed: Some(fixed.clone()),
                add_child: Some(callback!([fixed, tree_context, child_order] |child: &gtk4::Widget, child_node: Option<NodeId>| {
                    if child.parent().is_none() {
                        fixed.put(child, 0.0, 0.0);
                    }
                    let predecessor = child_order.borrow().last_key();
                    child_order.borrow_mut().place(child.clone(), child_node, predecessor);
                    let nodes = child_order.borrow().taffy_nodes();
                    tree_context.set_children(node_id, &nodes);
                    tree_context.refresh();
                })),
                insert_child: Some(callback!([fixed, tree_context, child_order] |child: &gtk4::Widget, child_node: Option<NodeId>, predecessor: Option<gtk4::Widget>| {
                    if child.parent().is_none() {
                        fixed.put(child, 0.0, 0.0);
                    }
                    child_order.borrow_mut().place(child.clone(), child_node, predecessor);
                    let nodes = child_order.borrow().taffy_nodes();
                    tree_context.set_children(node_id, &nodes);
                    tree_context.refresh();
                })),
                remove_child: Some(callback!([fixed, tree_context, child_order] |child: &gtk4::Widget, _: Option<NodeId>| {
                    if child.parent().as_ref() == Some(fixed.upcast_ref()) {
                        fixed.remove(child);
                    }
                    child_order.borrow_mut().remove(child.clone());
                    let nodes = child_order.borrow().taffy_nodes();
                    tree_context.set_children(node_id, &nodes);
                    tree_context.refresh();
                })),
                parent_node: Some(node_id),
            }) {
                $(props.children.clone())
            }
        }
    }
}
