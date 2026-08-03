use std::{cell::RefCell, rc::Rc};

use nestix::{
    Element, callback, closure, component, components::ContextProvider, layout, scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, ChildOrder, Color, FlexViewProps, Length, Rect, ResolvedStyle, StyleContext,
    StyleScope, TreeContext, WithAuto, matched_style, resolved_flex_view_style, style_align_items,
    style_align_self, style_flex_basis, style_flex_direction, style_flex_grow, style_flex_shrink,
    style_flex_wrap, style_gap, style_justify_content, style_length_with_auto, style_margin,
    style_padding,
};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSLayoutConstraint, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectView, NSWindingRule, NSWindowOrderingMode,
};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};
use taffy::{NodeId, Size, Style};

use crate::{WindowContext, contexts::ParentContext, material::visual_effect_view};
use nestix_native_core::utils::{gap_to_taffy, inset_to_taffy, margin_to_taffy, padding_to_taffy};

#[component]
pub fn FlexView(props: &FlexViewProps, element: &Element) -> Element {
    require_visual_mount!(element, FlexView, output);
    const DEFAULT_CLASSES: [&str; 2] = ["__FlexView", "__appkit_FlexView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();

    let mtm = MainThreadMarker::new().unwrap();
    let view = NNFlexView::new(
        mtm,
        FlexViewState {
            decoration: RefCell::new(None),
            material: RefCell::new(None),
        },
    );
    element.provide_handle(view.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(false);
    element.on_place(closure!(
        [view, parent_context] | placement | {
            parent_context.place_child(&view, Some(node_id), placement);
        }
    ));

    let matched_style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_flex_view_style(matched_style_props.clone(), props);
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

    scoped_effect!(
        [view, props.material, props.material_source] || {
            if let Some(previous) = view.ivars().material.borrow_mut().take() {
                previous.removeFromSuperview();
            }

            if let Some(material) = material.get()
                && let Some(effect) = visual_effect_view(
                    mtm,
                    material,
                    material_source.get(),
                    NSVisualEffectBlendingMode::WithinWindow,
                )
            {
                view.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, None);
                pin_to_bounds(&view, &effect);
                view.ivars().material.replace(Some(effect));
            }
        }
    );

    scoped_effect!(
        [view, style_props, window_context.scale_factor] || {
            let decoration_style =
                DecorationStyle::from_resolved(style_props.get().as_ref(), scale_factor.get());
            if decoration_style.is_visible() {
                if view.ivars().decoration.borrow().is_none() {
                    let decoration = NNFlexDecorationView::new(mtm);
                    if let Some(material) = view.ivars().material.borrow().as_ref() {
                        view.addSubview_positioned_relativeTo(
                            &decoration,
                            NSWindowOrderingMode::Above,
                            Some(material),
                        );
                    } else {
                        view.addSubview_positioned_relativeTo(
                            &decoration,
                            NSWindowOrderingMode::Below,
                            None,
                        );
                    }
                    pin_to_bounds(&view, &decoration);
                    view.ivars().decoration.replace(Some(decoration));
                }
                if let Some(decoration) = view.ivars().decoration.borrow().as_ref() {
                    decoration.set_style(decoration_style);
                }
            } else if let Some(decoration) = view.ivars().decoration.take() {
                decoration.removeFromSuperview();
            }
        }
    );

    scoped_effect!(
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
            let width = style_length_with_auto(
                style_props.as_ref(),
                width.get(),
                WithAuto::Auto,
                |style| style.width,
            );
            let height = style_length_with_auto(
                style_props.as_ref(),
                height.get(),
                WithAuto::Auto,
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
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.position,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_length_with_auto(style_props.as_ref(), left.get(), WithAuto::Auto, |style| {
                    style.left
                });
            let top =
                style_length_with_auto(style_props.as_ref(), top.get(), WithAuto::Auto, |style| {
                    style.top
                });

            tree_context.update_style(node_id, |prev| Style {
                position: nestix_native_core::style_position(style_props.as_ref(), position.get())
                    .to_taffy(),
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
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
            .effective_style = effective_style,
        ) {
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: Some(callback!([tree_context, view, child_order] |object: &NSObject,
                    child_node: Option<NodeId> | {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        let pointer = std::ptr::from_ref(object);
                        let predecessor = child_order.borrow().last_key();
                        child_order
                            .borrow_mut()
                            .place(pointer, child_node, predecessor);
                        view.addSubview(subview);
                        let nodes = child_order.borrow().taffy_nodes();
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    insert_child: Some(callback!([tree_context, view, child_order] |object: &NSObject,
                    child_node: Option<NodeId>,
                    predecessor: Option<*const NSObject> | {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        let pointer = std::ptr::from_ref(object);
                        child_order
                            .borrow_mut()
                            .place(pointer, child_node, predecessor);
                        let nodes = child_order.borrow().taffy_nodes();
                        view.addSubview(subview);
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    remove_child: Some(callback!([tree_context, child_order] |object: &NSObject,
                    _: Option<NodeId> | {
                        let subview = object.downcast_ref::<NSView>().unwrap();
                        subview.removeFromSuperview();
                        let pointer = std::ptr::from_ref(object);
                        child_order.borrow_mut().remove(pointer);
                        let nodes = child_order.borrow().taffy_nodes();
                        tree_context.set_children(node_id, &nodes);
                        tree_context.refresh();
                    })),
                    parent_node: Some(node_id)
                },
            ) {
                $(props.children.clone())
            }
        }
    }
}

struct FlexViewState {
    decoration: RefCell<Option<Retained<NNFlexDecorationView>>>,
    material: RefCell<Option<Retained<NSVisualEffectView>>>,
}

#[derive(Debug, Clone, PartialEq)]
struct DecorationStyle {
    background_color: Option<Color>,
    border_color: Option<Color>,
    widths: Rect<f64>,
    radius: f64,
}

impl Default for DecorationStyle {
    fn default() -> Self {
        Self {
            background_color: None,
            border_color: None,
            widths: Rect {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            radius: 0.0,
        }
    }
}

impl DecorationStyle {
    fn from_resolved(style: Option<&ResolvedStyle>, scale_factor: f64) -> Self {
        let logical = |length: Option<Length>| {
            length
                .map(|length| length.to_logical::<f64>(scale_factor).0)
                .unwrap_or(0.0)
                .max(0.0)
        };
        Self {
            background_color: style.and_then(|style| style.bg_color),
            border_color: style.and_then(|style| style.border_color),
            widths: Rect {
                top: logical(style.and_then(|style| style.border_top_width)),
                right: logical(style.and_then(|style| style.border_right_width)),
                bottom: logical(style.and_then(|style| style.border_bottom_width)),
                left: logical(style.and_then(|style| style.border_left_width)),
            },
            radius: logical(style.and_then(|style| style.border_radius)),
        }
    }

    fn is_visible(&self) -> bool {
        self.background_color.is_some()
            || self.border_color.is_some()
                && [
                    self.widths.top,
                    self.widths.right,
                    self.widths.bottom,
                    self.widths.left,
                ]
                .into_iter()
                .any(|width| width > 0.0)
    }
}

struct FlexDecorationState {
    style: RefCell<DecorationStyle>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FlexDecorationState]
    struct NNFlexDecorationView;

    unsafe impl NSObjectProtocol for NNFlexDecorationView {}

    impl NNFlexDecorationView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> {
            None
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let bounds = self.bounds();
            let width = bounds.size.width.max(0.0);
            let height = bounds.size.height.max(0.0);
            if width == 0.0 || height == 0.0 {
                return;
            }

            let style = self.ivars().style.borrow();
            let radius = style.radius.min(width / 2.0).min(height / 2.0);
            let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                bounds, radius, radius,
            );

            if let Some(color) = style.background_color {
                ns_color(color).setFill();
                outline.fill();
            }

            let Some(color) = style.border_color else {
                return;
            };
            let top = style.widths.top.min(height);
            let right = style.widths.right.min(width);
            let bottom = style.widths.bottom.min(height);
            let left = style.widths.left.min(width);
            if top == 0.0 && right == 0.0 && bottom == 0.0 && left == 0.0 {
                return;
            }

            let border = NSBezierPath::bezierPath();
            border.appendBezierPath(&outline);
            if let Some(inner) = inner_border_geometry(bounds, &style.widths, radius) {
                border.appendBezierPath(&rounded_rect_path(inner));
                border.setWindingRule(NSWindingRule::EvenOdd);
            }
            ns_color(color).setFill();
            border.fill();
        }
    }
);

#[derive(Debug, Clone, Copy, PartialEq)]
struct CornerRadius {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RoundedRect {
    rect: NSRect,
    top_left: CornerRadius,
    top_right: CornerRadius,
    bottom_right: CornerRadius,
    bottom_left: CornerRadius,
}

fn inner_border_geometry(
    bounds: NSRect,
    widths: &Rect<f64>,
    outer_radius: f64,
) -> Option<RoundedRect> {
    let width = bounds.size.width.max(0.0);
    let height = bounds.size.height.max(0.0);
    let top = widths.top.clamp(0.0, height);
    let right = widths.right.clamp(0.0, width);
    let bottom = widths.bottom.clamp(0.0, height);
    let left = widths.left.clamp(0.0, width);
    let inner_width = width - left - right;
    let inner_height = height - top - bottom;
    if inner_width <= 0.0 || inner_height <= 0.0 {
        return None;
    }

    let mut top_left = CornerRadius {
        x: (outer_radius - left).max(0.0),
        y: (outer_radius - top).max(0.0),
    };
    let mut top_right = CornerRadius {
        x: (outer_radius - right).max(0.0),
        y: (outer_radius - top).max(0.0),
    };
    let mut bottom_right = CornerRadius {
        x: (outer_radius - right).max(0.0),
        y: (outer_radius - bottom).max(0.0),
    };
    let mut bottom_left = CornerRadius {
        x: (outer_radius - left).max(0.0),
        y: (outer_radius - bottom).max(0.0),
    };

    let ratio = |available: f64, combined: f64| {
        if combined > available && combined > 0.0 {
            available / combined
        } else {
            1.0
        }
    };
    let scale = ratio(inner_width, top_left.x + top_right.x)
        .min(ratio(inner_width, bottom_left.x + bottom_right.x))
        .min(ratio(inner_height, top_left.y + bottom_left.y))
        .min(ratio(inner_height, top_right.y + bottom_right.y));
    for radius in [
        &mut top_left,
        &mut top_right,
        &mut bottom_right,
        &mut bottom_left,
    ] {
        radius.x *= scale;
        radius.y *= scale;
    }

    Some(RoundedRect {
        rect: NSRect::new(
            NSPoint::new(bounds.origin.x + left, bounds.origin.y + top),
            NSSize::new(inner_width, inner_height),
        ),
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    })
}

fn rounded_rect_path(rounded: RoundedRect) -> Retained<NSBezierPath> {
    const KAPPA: f64 = 0.552_284_749_830_793_6;

    let path = NSBezierPath::bezierPath();
    let x0 = rounded.rect.origin.x;
    let y0 = rounded.rect.origin.y;
    let x1 = x0 + rounded.rect.size.width;
    let y1 = y0 + rounded.rect.size.height;
    let tl = rounded.top_left;
    let tr = rounded.top_right;
    let br = rounded.bottom_right;
    let bl = rounded.bottom_left;

    path.moveToPoint(NSPoint::new(x0 + tl.x, y0));
    path.lineToPoint(NSPoint::new(x1 - tr.x, y0));
    path.curveToPoint_controlPoint1_controlPoint2(
        NSPoint::new(x1, y0 + tr.y),
        NSPoint::new(x1 - tr.x * (1.0 - KAPPA), y0),
        NSPoint::new(x1, y0 + tr.y * (1.0 - KAPPA)),
    );
    path.lineToPoint(NSPoint::new(x1, y1 - br.y));
    path.curveToPoint_controlPoint1_controlPoint2(
        NSPoint::new(x1 - br.x, y1),
        NSPoint::new(x1, y1 - br.y * (1.0 - KAPPA)),
        NSPoint::new(x1 - br.x * (1.0 - KAPPA), y1),
    );
    path.lineToPoint(NSPoint::new(x0 + bl.x, y1));
    path.curveToPoint_controlPoint1_controlPoint2(
        NSPoint::new(x0, y1 - bl.y),
        NSPoint::new(x0 + bl.x * (1.0 - KAPPA), y1),
        NSPoint::new(x0, y1 - bl.y * (1.0 - KAPPA)),
    );
    path.lineToPoint(NSPoint::new(x0, y0 + tl.y));
    path.curveToPoint_controlPoint1_controlPoint2(
        NSPoint::new(x0 + tl.x, y0),
        NSPoint::new(x0, y0 + tl.y * (1.0 - KAPPA)),
        NSPoint::new(x0 + tl.x * (1.0 - KAPPA), y0),
    );
    path.closePath();
    path
}

impl NNFlexDecorationView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FlexDecorationState {
            style: RefCell::new(DecorationStyle::default()),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn set_style(&self, style: DecorationStyle) {
        if *self.ivars().style.borrow() != style {
            self.ivars().style.replace(style);
            self.setNeedsDisplay(true);
        }
    }
}

fn ns_color(color: Color) -> Retained<NSColor> {
    let rgb = color.into_rgb();
    NSColor::colorWithDeviceRed_green_blue_alpha(
        f64::from(rgb.red) / 255.0,
        f64::from(rgb.green) / 255.0,
        f64::from(rgb.blue) / 255.0,
        f64::from(rgb.alpha) / 255.0,
    )
}

fn pin_to_bounds(parent: &NSView, child: &NSView) {
    child.setTranslatesAutoresizingMaskIntoConstraints(false);
    let constraints = NSArray::from_retained_slice(&[
        child
            .topAnchor()
            .constraintEqualToAnchor(&parent.topAnchor()),
        child
            .bottomAnchor()
            .constraintEqualToAnchor(&parent.bottomAnchor()),
        child
            .leadingAnchor()
            .constraintEqualToAnchor(&parent.leadingAnchor()),
        child
            .trailingAnchor()
            .constraintEqualToAnchor(&parent.trailingAnchor()),
    ]);
    NSLayoutConstraint::activateConstraints(&constraints);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoration_style_resolves_and_clamps_border_lengths() {
        let mut style = ResolvedStyle::default();
        style.border_color = Some(Color::RED);
        style.border_radius = Some(Length::physical(4));
        style.border_left_width = Some(Length::logical(-2));
        style.border_right_width = Some(Length::physical(6));
        style.border_top_width = Some(Length::logical(1));

        let decoration = DecorationStyle::from_resolved(Some(&style), 2.0);

        assert_eq!(decoration.radius, 2.0);
        assert_eq!(decoration.widths.left, 0.0);
        assert_eq!(decoration.widths.right, 3.0);
        assert_eq!(decoration.widths.top, 1.0);
        assert!(decoration.is_visible());
    }

    #[test]
    fn widths_without_a_color_do_not_create_a_decoration() {
        let mut style = ResolvedStyle::default();
        style.border_left_width = Some(Length::logical(2));

        assert!(!DecorationStyle::from_resolved(Some(&style), 1.0).is_visible());
    }

    #[test]
    fn inner_border_curve_accounts_for_each_adjacent_edge() {
        let inner = inner_border_geometry(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 50.0)),
            &Rect {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            10.0,
        )
        .unwrap();

        assert_eq!(inner.rect.origin, NSPoint::new(4.0, 1.0));
        assert_eq!(inner.rect.size, NSSize::new(94.0, 46.0));
        assert_eq!(inner.top_left, CornerRadius { x: 6.0, y: 9.0 });
        assert_eq!(inner.top_right, CornerRadius { x: 8.0, y: 9.0 });
        assert_eq!(inner.bottom_right, CornerRadius { x: 8.0, y: 7.0 });
        assert_eq!(inner.bottom_left, CornerRadius { x: 6.0, y: 7.0 });
    }
}
