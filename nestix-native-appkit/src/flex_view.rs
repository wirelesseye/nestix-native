use std::cell::RefCell;

use nestix::{Element, callback, closure, component, components::ContextProvider, effect, layout};
use nestix_native_core::{
    Alignment, Direction, ExtendsViewProps, FlexViewProps, TreeContext, Wrap,
};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSBox, NSBoxType, NSColor, NSLayoutConstraint, NSView};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style};

use crate::{WindowContext, contexts::ParentContext};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();
    let view = NNFlexView::new(
        mtm,
        FlexViewState {
            ns_box: RefCell::new(None),
        },
    );
    element.provide_handle(view.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(false);
    if let Some(add_child) = &parent_context.add_child {
        add_child(&view, Some(node_id));
    }

    effect!(
        [view, props.background_color] || {
            if let Some(background_color) = background_color.get() {
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

    effect!(
        [tree_context, props.grow()] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: grow.get(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
        [
            window_context,
            tree_context,
            parent_context.parent_node,
            props.width(),
            props.height(),
        ] || {
            let scale_factor = window_context.scale_factor.get();

            if parent_node.is_some() {
                // Update size when the node is not root
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.get().into_taffy_dimension(scale_factor),
                        height: height.get().into_taffy_dimension(scale_factor),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    effect!(
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

    effect!(
        [tree_context, props.alignment] || {
            tree_context.update_style(node_id, |prev| Style {
                align_items: match alignment.get() {
                    Alignment::Unset => None,
                    Alignment::FlexStart => Some(taffy::AlignItems::FlexStart),
                    Alignment::FlexEnd => Some(taffy::AlignItems::FlexEnd),
                    Alignment::Center => Some(taffy::AlignItems::Center),
                },
                ..prev
            });

            tree_context.refresh();
        }
    );

    effect!(
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

    effect!(
        [tree_context, parent_context.parent_node, view] || {
            if parent_node.is_some() {
                if let Some(layout) = tree_context.layout(node_id) {
                    view.setFrame(NSRect::new(
                        NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                        NSSize::new(layout.size.width.into(), layout.size.height.into()),
                    ));
                }
            }
        }
    );

    element.on_destroy(closure!(
        [view] || {
            view.removeFromSuperview();
        }
    ));

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                add_child: Some(callback!([tree_context, view] |object: &NSObject, child_node: Option<NodeId>| {
                    view.addSubview(object.downcast_ref::<NSView>().unwrap());
                    if let Some(child_node) = child_node {
                        tree_context.add_child(node_id, child_node);
                    }
                })),
                remove_child: Some(callback!([tree_context] |object: &NSObject, child_node: Option<NodeId>| {
                    object.downcast_ref::<NSView>().unwrap().removeFromSuperview();
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
