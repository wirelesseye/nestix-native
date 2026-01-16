use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, Shared, closure, component, effect, prop::PropValue};
use nestix_native_core::{ExtendsViewProps, InputProps, Length};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSSize, NSString};

use crate::{ParentContext, WindowContext};

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<InputDelegate>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();
    let string_value = NSString::from_str(&props.value.get());
    let input = NSTextField::textFieldWithString(&string_value, mtm);

    let delegate = InputDelegate::new(
        mtm,
        InputState {
            on_text_change: props.on_text_change.clone(),
        },
    );
    unsafe {
        input.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }

    let input_id = nanoid::nanoid!();

    element.on_destroy(closure!(input_id, input => || {
        input.removeFromSuperview();
        DELEGATES.with_borrow_mut(|delegates| delegates.remove(&input_id));
    }));

    DELEGATES.with_borrow_mut(|delegates| delegates.insert(input_id, delegate));

    element.provide_handle(input.as_ref() as *const NSObject);

    effect!(input, props.value => || {
        let string_value = NSString::from_str(&value.get());
        input.setStringValue(&string_value);
    });

    effect!(input, window_context.scale_factor, props.x(), props.y() => || {
        let scale_factor = scale_factor.get();
        let x: f64 = match x.get() {
            Length::Auto => 0.0,
            Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
        };
        let y: f64 = match y.get() {
            Length::Auto => 0.0,
            Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
        };
        input.setFrameOrigin(NSPoint::new(x, y));
    });

    effect!(input, window_context.scale_factor, props.width(), props.height() => || {
        let scale_factor = scale_factor.get();
        let width: f64 = match width.get() {
            Length::Auto => 0.0,
            Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
        };
        let height: f64 = match height.get() {
            Length::Auto => 0.0,
            Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
        };
        input.setFrameSize(NSSize::new(width, height));
    });

    let parent = element.context::<ParentContext>();
    if let Some(parent) = parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&input);
        }
    }
}

#[derive(Debug)]
struct InputState {
    on_text_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
}

define_class! {
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
}

impl InputDelegate {
    fn new(mtm: MainThreadMarker, state: InputState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
