use std::{cell::RefCell, rc::Rc};

use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    ChildOrder, Dimension, FlexViewProps, StyleContext, StyleScope, TreeContext, matched_style,
    resolved_flex_view_style, style_align_items, style_align_self, style_dimension,
    style_flex_basis, style_flex_direction, style_flex_grow, style_flex_shrink, style_flex_wrap,
    style_gap, style_justify_content, style_margin, style_padding,
};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSBox, NSBoxType, NSColor, NSLayoutConstraint, NSView};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style};

use crate::{WindowContext, contexts::ParentContext};
use nestix_native_core::utils::{gap_to_taffy, inset_to_taffy, margin_to_taffy, padding_to_taffy};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__appkit_FlexView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();

    let mtm = MainThreadMarker::new().unwrap();
    let view = NNFlexView::new(
        mtm,
        FlexViewState {
            ns_box: RefCell::new(None),
        },
    );
    element.provide_handle(view.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(false);
    element.on_place(closure!(
        [view, parent_context] | placement | {
            parent_context.place_child(&view, Some(node_id), placement);
        }
    ));

    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(style_props.clone(), props);

    scoped_effect!(
        element,
        [view, style_props, props.bg_color] || {
            let style_props = style_props.get();
            let bg_color = bg_color.get().or_else(|| {
                style_props
                    .as_ref()
                    .and_then(|style_props| style_props.bg_color)
            });
            if let Some(background_color) = bg_color {
                if view.ivars().ns_box.borrow().is_none() {
                    let ns_box = NSBox::new(mtm);
                    ns_box.setBoxType(NSBoxType::Custom);
                    ns_box.setBorderWidth(0.0);

                    view.addSubview(&ns_box);
                    ns_box.setTranslatesAutoresizingMaskIntoConstraints(false);
                    let constraints = NSArray::from_retained_slice(&[
                        ns_box
                            .topAnchor()
                            .constraintEqualToAnchor(&view.topAnchor()),
                        ns_box
                            .bottomAnchor()
                            .constraintEqualToAnchor(&view.bottomAnchor()),
                        ns_box
                            .leadingAnchor()
                            .constraintEqualToAnchor(&view.leadingAnchor()),
                        ns_box
                            .trailingAnchor()
                            .constraintEqualToAnchor(&view.trailingAnchor()),
                    ]);
                    NSLayoutConstraint::activateConstraints(&constraints);

                    view.ivars().ns_box.replace(Some(ns_box));
                }
                let ns_box = view.ivars().ns_box.borrow();
                let ns_box = ns_box.as_ref().unwrap();
                let rgb = background_color.into_rgb();
                let fill_color = NSColor::colorWithDeviceRed_green_blue_alpha(
                    rgb.red as f64 / 255.0,
                    rgb.green as f64 / 255.0,
                    rgb.blue as f64 / 255.0,
                    rgb.alpha as f64 / 255.0,
                );
                ns_box.setFillColor(&fill_color);
            } else {
                if let Some(ns_box) = view.ivars().ns_box.take() {
                    ns_box.removeFromSuperview();
                }
            }
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.width,
            props.view.height,
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

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
                style.top
            });

            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.container.padding()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                padding: padding_to_taffy(
                    style_padding(style_props.as_ref(), padding.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.gap
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let gap = gap_to_taffy(style_gap(style_props.as_ref(), gap.get()), scale_factor);

            tree_context.update_style(node_id, |prev| Style {
                gap: Size {
                    width: gap,
                    height: gap,
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.flex_direction] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: style_flex_direction(style_props.as_ref(), flex_direction.get())
                    .to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.align_items] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_items: style_align_items(style_props.as_ref(), align_items.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.justify_content] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                justify_content: style_justify_content(style_props.as_ref(), justify_content.get())
                    .to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.flex_wrap] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: style_flex_wrap(style_props.as_ref(), flex_wrap.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, view] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                view.setFrame(NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                ));
            }
        }
    );

    element.on_unmount(closure!(
        [view] || {
            view.removeFromSuperview();
        }
    ));

    let child_order = Rc::new(RefCell::new(ChildOrder::<*const NSObject>::new()));

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style
        ) {
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: Some(callback!([tree_context, view, child_order] |object: &NSObject, child_node: Option<NodeId>| {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        let pointer = std::ptr::from_ref(object);
                        let predecessor = child_order.borrow().last_key();
                        child_order.borrow_mut().place(pointer, child_node, predecessor);
                        view.addSubview(subview);
                        let nodes = child_order.borrow().taffy_nodes();
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    insert_child: Some(callback!([tree_context, view, child_order]
                        |object: &NSObject, child_node: Option<NodeId>, predecessor: Option<*const NSObject>| {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        let pointer = std::ptr::from_ref(object);
                        child_order.borrow_mut().place(pointer, child_node, predecessor);
                        let nodes = child_order.borrow().taffy_nodes();
                        view.addSubview(subview);
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    remove_child: Some(callback!([tree_context, child_order] |object: &NSObject, _: Option<NodeId>| {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        subview.removeFromSuperview();
                        let pointer = std::ptr::from_ref(object);
                        child_order.borrow_mut().remove(pointer);
                        let nodes = child_order.borrow().taffy_nodes();
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    parent_node: Some(node_id),
                }
            ) {
                $(props.children.clone())
            }
        }
    }
}

struct FlexViewState {
    ns_box: RefCell<Option<Retained<NSBox>>>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FlexViewState]
    struct NNFlexView;

    unsafe impl NSObjectProtocol for NNFlexView {}

    impl NNFlexView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl AsRef<NSObject> for NNFlexView {
    fn as_ref(&self) -> &NSObject {
        &self
    }
}

impl NNFlexView {
    fn new(mtm: MainThreadMarker, state: FlexViewState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
