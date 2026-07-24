use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, create_state, scoped_effect};
use nestix_native_core::{StyleContext, SwitchProps, matched_style};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSSwitch};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::native_control;

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<SwitchHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Switch(props: &SwitchProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Switch", "__appkit_Switch"];

    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().unwrap();
    let handler = SwitchHandler::new(
        mtm,
        SwitchHandlerState {
            on_checked_change: props.on_checked_change.clone(),
        },
    );
    let switch = NSSwitch::new(mtm);
    unsafe {
        switch.setTarget(Some(&handler));
        switch.setAction(Some(sel!(changed:)));
    }

    let handler_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler));
    element.on_unmount(closure!(
        [handler_id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    native_control::mount(
        element,
        switch.clone().into_super().into_super(),
        style_props,
        &props.view,
        create_state(0usize).into_readonly(),
    );

    scoped_effect!(
        [switch, props.enabled, props.checked] || {
            switch.setEnabled(enabled.get());
            switch.setState(if checked.get() {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    );
}

#[derive(Debug)]
struct SwitchHandlerState {
    on_checked_change: PropValue<Option<Shared<dyn Fn(bool)>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SwitchHandlerState]
    #[derive(Debug)]
    struct SwitchHandler;

    unsafe impl NSObjectProtocol for SwitchHandler {}

    impl SwitchHandler {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSSwitch) {
            if let Some(callback) = self.ivars().on_checked_change.get() {
                callback(sender.state() == NSControlStateValueOn);
            }
        }
    }
);

impl SwitchHandler {
    fn new(mtm: MainThreadMarker, state: SwitchHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
