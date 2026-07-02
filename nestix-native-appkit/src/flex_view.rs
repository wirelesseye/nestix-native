use std::cell::RefCell;

use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension, Direction, FlexViewProps, StyleContext, TreeContext, Wrap, matched_style,
    style_align_self, style_dimension, style_grow, style_margin,
};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSBox, NSBoxType, NSColor, NSLayoutConstraint, NSView};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style};

use crate::{WindowContext, contexts::ParentContext, utils::margin_to_taffy};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
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
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&view, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&view, Some(node_id));
            }
        }
    ));

    let style_props = matched_style(
        style_context,
        props.class.clone(),
        &["__FlexView", "__appkit_FlexView"],
    );

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
        [tree_context, style_props, props.view.grow] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_grow(style_props.as_ref(), grow.get()),
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
        [tree_context, props.direction] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_direction: match direction.get() {
                    Direction::Row => taffy::FlexDirection::Row,
                    Direction::RowReverse => taffy::FlexDirection::RowReverse,
                    Direction::Column => taffy::FlexDirection::Column,
                    Direction::ColumnReverse => taffy::FlexDirection::ColumnReverse,
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, props.align_items] || {
            tree_context.update_style(node_id, |prev| Style {
                align_items: align_items.get().to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, props.wrap] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_wrap: match wrap.get() {
                    Wrap::NoWrap => taffy::FlexWrap::NoWrap,
                    Wrap::Wrap => taffy::FlexWrap::Wrap,
                },
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

    layout! {
        ContextProvider<ParentContext>(
            ParentContext {
                add_child: Some(callback!([tree_context, view] |object: &NSObject, child_node: Option<NodeId>| {
                    let subview = object.downcast_ref::<NSView>().unwrap();
                    if view.subviews().containsObject(subview) {
                        subview.removeFromSuperview();
                        if let Some(child_node) = child_node {
                            tree_context.remove_child(node_id, child_node);
                        }
                    }
                    view.addSubview(subview);
                    if let Some(child_node) = child_node {
                        tree_context.add_child(node_id, child_node);
                        tree_context.refresh();
                    }
                })),
                insert_child: Some(callback!([tree_context, view]
                    |object: &NSObject, child_node: Option<NodeId>, index: usize| {
                    let subview = object.downcast_ref::<NSView>().unwrap();
                    if view.subviews().containsObject(subview) {
                        subview.removeFromSuperview();
                        if let Some(child_node) = child_node {
                            tree_context.remove_child(node_id, child_node);
                        }
                    }
                    view.addSubview(subview);
                    if let Some(child_node) = child_node {
                        tree_context.insert_child(node_id, child_node, index);
                        tree_context.refresh();
                    }
                })),
                remove_child: Some(callback!([tree_context] |object: &NSObject, child_node: Option<NodeId>| {
                    let subview = object.downcast_ref::<NSView>().unwrap();
                    subview.removeFromSuperview();
                    if let Some(child_node) = child_node {
                        tree_context.remove_child(node_id, child_node);
                    }
                })),
                parent_node: Some(node_id),
            }
        ) {
            $(props.children.clone())
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
