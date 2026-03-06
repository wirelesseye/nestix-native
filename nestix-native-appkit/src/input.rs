use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, effect};
use nestix_native_core::{Dimension, ExtendsViewProps, InputProps};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate};
use objc2_foundation::{
    NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use taffy::{Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{ParentContext, TreeContext},
};

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<InputDelegate>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

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
    if let Some(add_child) = &parent_context.add_child {
        add_child(&input, Some(node_id));
    }

    element.on_destroy(closure!(
        [parent_context, input] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&input, Some(node_id));
            }
            DELEGATES.with_borrow_mut(|delegates| delegates.remove(&input_id));
        }
    ));

    effect!(
        [input, props.value] || {
            let string_value = NSString::from_str(&value.get());
            input.setStringValue(&string_value);
        }
    );

    // effect!(
    //     [
    //         input,
    //         window_context.scale_factor,
    //         props.left(),
    //         props.top()
    //     ] || {
    //         let scale_factor = scale_factor.get();
    //         let x: f64 = match left.get() {
    //             Dimension::Auto => 0.0,
    //             Dimension::Length(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
    //         };
    //         let y: f64 = match top.get() {
    //             Dimension::Auto => 0.0,
    //             Dimension::Length(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
    //         };
    //         input.setFrameOrigin(NSPoint::new(x, y));
    //     }
    // );

    effect!(
        [tree_context, props.grow()] || {
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: grow.get(),
                ..prev
            });

            tree_context.update();
        }
    );

    effect!(
        [
            tree_context,
            input,
            props.width(),
            props.height()
        ] || {
            let scale_factor = window_context.scale_factor.get();

            if width.get().is_auto() || height.get().is_auto() {
                input.sizeToFit();
            }
            let width = match width.get() {
                Dimension::Auto => input.frame().size.width as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height.get() {
                Dimension::Auto => input.frame().size.height as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };

            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width: taffy::Dimension::from_length(width),
                    height: taffy::Dimension::from_length(height),
                },
                ..prev
            });

            tree_context.update();
        }
    );

    effect!(
        [tree_context, input] || {
            if let Some(layout) = tree_context.layout(node_id) {
                input.setFrame(NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                ));
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
