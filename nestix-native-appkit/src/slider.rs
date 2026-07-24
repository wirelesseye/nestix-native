use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, create_state, scoped_effect};
use nestix_native_core::{SliderProps, StyleContext, matched_style};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::NSSlider;
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::native_control;

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<SliderHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Slider(props: &SliderProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Slider", "__appkit_Slider"];

    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().unwrap();
    let handler = SliderHandler::new(
        mtm,
        SliderHandlerState {
            on_value_change: props.on_value_change.clone(),
        },
    );
    let slider = unsafe {
        NSSlider::sliderWithValue_minValue_maxValue_target_action(
            props.value.get(),
            props.minimum.get(),
            props.maximum.get(),
            Some(&handler),
            Some(sel!(changed:)),
            mtm,
        )
    };

    let handler_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler));
    element.on_unmount(closure!(
        [handler_id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    native_control::mount(
        element,
        slider.clone().into_super().into_super(),
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
    );

    scoped_effect!(
        [
            slider,
            props.enabled,
            props.value,
            props.minimum,
            props.maximum
        ] || {
            slider.setEnabled(enabled.get());
            slider.setMinValue(minimum.get());
            slider.setMaxValue(maximum.get());
            slider.setDoubleValue(value.get());
        }
    );
}

#[derive(Debug)]
struct SliderHandlerState {
    on_value_change: PropValue<Option<Shared<dyn Fn(f64)>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SliderHandlerState]
    #[derive(Debug)]
    struct SliderHandler;

    unsafe impl NSObjectProtocol for SliderHandler {}

    impl SliderHandler {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSSlider) {
            if let Some(callback) = self.ivars().on_value_change.get() {
                callback(sender.doubleValue());
            }
        }
    }
);

impl SliderHandler {
    fn new(mtm: MainThreadMarker, state: SliderHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
