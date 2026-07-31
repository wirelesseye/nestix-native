use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{Element, PropValue, Shared, closure, component, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, InputProps, Length, StyleContext, TreeContext, WithAuto, matched_style,
    resolved_view_style, style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_length_with_auto, style_margin,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate};
use objc2_foundation::{
    NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<InputDelegate>>> = RefCell::new(HashMap::new());
}

const DEFAULT_INPUT_WIDTH: f32 = 200.0;
const DEFAULT_INPUT_HEIGHT: f32 = 22.0;

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    require_visual_mount!(element, Input);
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__appkit_Input"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let matched_style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_style = resolved_view_style(matched_style_props, &props.view);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        target_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, target_style, window_context.scale_factor] || {
            animated_style.set_target(target_style.get(), scale_factor.get());
        }
    );

    let mtm = MainThreadMarker::new().unwrap();
    let string_value = NSString::from_str(&props.value.get());
    let input = NSTextField::textFieldWithString(&string_value, mtm);
    let placeholder = NSString::from_str(&props.placeholder.get());
    input.setPlaceholderString(Some(&placeholder));
    element.provide_handle(input.as_ref() as *const NSObject);

    let input_id = nanoid::nanoid!();

    let delegate = InputDelegate::new(
        mtm,
        InputState {
            on_text_change: props.on_text_change.clone(),
        },
    );
    unsafe {
        input.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }
    DELEGATES.with_borrow_mut(|delegates| delegates.insert(input_id.clone(), delegate));

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [input, parent_context] | placement | {
            parent_context.place_child(&input, Some(node_id), placement);
        }
    ));

    element.on_unmount(closure!(
        [parent_context, input] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&input, Some(node_id));
            }
            DELEGATES.with_borrow_mut(|delegates| delegates.remove(&input_id));
        }
    ));

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
            input,
            props.view.width,
            props.view.height,
            props.value,
            props.placeholder,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let string_value = NSString::from_str(&value.get());
            input.setStringValue(&string_value);
            let placeholder = NSString::from_str(&placeholder.get());
            input.setPlaceholderString(Some(&placeholder));
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

            let intrinsic_size =
                (width.is_auto() || height.is_auto()).then(|| input.intrinsicContentSize());
            let (width, min_width) = input_dimension(
                width,
                intrinsic_size.map(|size| size.width as f32),
                DEFAULT_INPUT_WIDTH,
                scale_factor,
            );
            let (height, min_height) = input_dimension(
                height,
                intrinsic_size.map(|size| size.height as f32),
                DEFAULT_INPUT_HEIGHT,
                scale_factor,
            );

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size { width, height },
                    min_size: Size {
                        width: min_width,
                        height: min_height,
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
        [tree_context, parent_context.parent_node, input] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let alignment_rect = NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                );
                input.setFrame(input.frameForAlignmentRect(alignment_rect));
            }
        }
    );
}

fn input_dimension(
    value: WithAuto<Length>,
    intrinsic: Option<f32>,
    fallback: f32,
    scale_factor: f64,
) -> (taffy::Dimension, taffy::Dimension) {
    match value {
        WithAuto::Auto => match intrinsic.filter(|value| value.is_finite() && *value > 0.0) {
            Some(intrinsic) => (
                taffy::Dimension::from_length(intrinsic),
                taffy::Dimension::auto(),
            ),
            None => (
                taffy::Dimension::auto(),
                taffy::Dimension::from_length(fallback),
            ),
        },
        WithAuto::Value(value) => (
            taffy::Dimension::from_length(value.to_logical::<f32>(scale_factor)),
            taffy::Dimension::auto(),
        ),
    }
}

#[derive(Debug)]
struct InputState {
    on_text_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = InputState]
    #[derive(Debug)]
    struct InputDelegate;

    unsafe impl NSObjectProtocol for InputDelegate {}

    unsafe impl NSControlTextEditingDelegate for InputDelegate {}

    unsafe impl NSTextFieldDelegate for InputDelegate {}

    impl InputDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, notification: &NSNotification) {
            if let Some(on_text_change) = self.ivars().on_text_change.get() {
                if let Some(object) = notification.object() {
                    let text_field = object.downcast_ref::<NSTextField>().unwrap();
                    let value = text_field.stringValue();
                    on_text_change(&value.to_string());
                }
            }
        }
    }
);

impl InputDelegate {
    fn new(mtm: MainThreadMarker, state: InputState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::{FlexDirection, TaffyTree};

    #[test]
    fn auto_input_width_uses_a_minimum_and_stretches() {
        let (width, min_width) = input_dimension(WithAuto::Auto, Some(-1.0), 200.0, 1.0);
        let (height, min_height) = input_dimension(WithAuto::Auto, Some(22.0), 22.0, 1.0);
        let mut tree = TaffyTree::<()>::new();
        let input = tree
            .new_leaf(Style {
                size: Size { width, height },
                min_size: Size {
                    width: min_width,
                    height: min_height,
                },
                ..Style::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    size: Size {
                        width: taffy::Dimension::from_length(600.0),
                        height: taffy::Dimension::from_length(100.0),
                    },
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                },
                &[input],
            )
            .unwrap();

        tree.compute_layout(root, Size::max_content()).unwrap();
        let layout = tree.layout(input).unwrap();
        assert_eq!(layout.size.width, 600.0);
        assert_eq!(layout.size.height, 22.0);
    }
}
