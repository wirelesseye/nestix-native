use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, scoped_effect};
use nestix_native_core::{
    Dimension, InputProps, StyleContext, TreeContext, matched_style, style_align_self,
    style_dimension, style_grow, style_margin,
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

use crate::{WindowContext, contexts::ParentContext, utils::margin_to_taffy};

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<InputDelegate>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        props.class.clone(),
        &["__Input", "__appkit_Input"],
    );

    let mtm = MainThreadMarker::new().unwrap();
    let string_value = NSString::from_str(&props.value.get());
    let input = NSTextField::textFieldWithString(&string_value, mtm);
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
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&input, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&input, Some(node_id));
            }
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
            input,
            props.view.width,
            props.view.height,
            props.value,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let string_value = NSString::from_str(&value.get());
            input.setStringValue(&string_value);
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

            let intrinsic_size =
                (width.is_auto() || height.is_auto()).then(|| input.intrinsicContentSize());
            let width = match width {
                Dimension::Auto => intrinsic_size.unwrap().width as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height {
                Dimension::Auto => intrinsic_size.unwrap().height as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
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
