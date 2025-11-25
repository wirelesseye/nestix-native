use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, Shared, closure, component, effect, prop::PropValue};
use nestix_native_core::InputProps;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSSize, NSString};

use crate::ParentContext;

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<InputDelegate>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Input(props: &InputProps, element: &Element) {
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

    element.on_destroy(closure!(input_id => || {
        DELEGATES.with_borrow_mut(|delegates| delegates.remove(&input_id));
    }));

    DELEGATES.with_borrow_mut(|delegates| delegates.insert(input_id, delegate));

    element.provide_handle(input.as_ref() as *const NSObject);

    effect!(input, props.value => || {
        let string_value = NSString::from_str(&value.get());
        input.setStringValue(&string_value);
    });

    effect!(input, props.x, props.y => || {
        input.setFrameOrigin(NSPoint::new(x.get(), y.get()));
    });

    effect!(input, props.width, props.height => || {
        input.setFrameSize(NSSize::new(width.get(), height.get()));
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
