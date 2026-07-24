use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, create_state, scoped_effect};
use nestix_native_core::{CheckboxProps, StyleContext, matched_style, resolve_font_props};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{NSButton, NSControlStateValueOff, NSControlStateValueOn};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

use crate::{
    font::{ns_color, resolve_font},
    native_control,
};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<CheckboxHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Checkbox(props: &CheckboxProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Checkbox", "__appkit_Checkbox"];

    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().unwrap();
    let handler = CheckboxHandler::new(
        mtm,
        CheckboxHandlerState {
            on_checked_change: props.on_checked_change.clone(),
        },
    );
    let checkbox = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(&props.title.get()),
            Some(&handler),
            Some(sel!(changed:)),
            mtm,
        )
    };
    let original_font = checkbox.font().unwrap();
    let original_color = checkbox.contentTintColor();

    let handler_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler));
    element.on_unmount(closure!(
        [handler_id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    let content_revision = create_state(0usize);
    native_control::mount(
        element,
        checkbox.clone().into_super().into_super(),
        style_props.clone(),
        &props.view,
        content_revision.clone().into_readonly(),
    );

    scoped_effect!(
        [
            checkbox,
            style_props,
            props.title,
            props.enabled,
            props.checked,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            original_font,
            original_color,
            content_revision
        ] || {
            checkbox.setTitle(&NSString::from_str(&title.get()));
            checkbox.setEnabled(enabled.get());
            checkbox.setState(if checked.get() {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            let font_props = resolve_font_props(
                style_props.get().as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            checkbox.setFont(Some(&resolve_font(&original_font, &font_props, mtm)));
            if let Some(color) = font_props.text_color {
                checkbox.setContentTintColor(Some(&ns_color(color)));
            } else {
                checkbox.setContentTintColor(original_color.as_deref());
            }
            content_revision.mutate(|revision| *revision = revision.wrapping_add(1));
        }
    );
}

#[derive(Debug)]
struct CheckboxHandlerState {
    on_checked_change: PropValue<Option<Shared<dyn Fn(bool)>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = CheckboxHandlerState]
    #[derive(Debug)]
    struct CheckboxHandler;

    unsafe impl NSObjectProtocol for CheckboxHandler {}

    impl CheckboxHandler {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSButton) {
            if let Some(callback) = self.ivars().on_checked_change.get() {
                callback(sender.state() == NSControlStateValueOn);
            }
        }
    }
);

impl CheckboxHandler {
    fn new(mtm: MainThreadMarker, state: CheckboxHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
