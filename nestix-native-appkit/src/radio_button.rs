use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, create_state, scoped_effect};
use nestix_native_core::{RadioButtonProps, StyleContext, matched_style, resolve_font_props};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{NSButton, NSControlStateValueOff, NSControlStateValueOn, NSWindow};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

use crate::{
    WindowContext,
    font::{ns_color, resolve_font},
    native_control,
};

struct RegisteredRadioButton {
    group: PropValue<String>,
    button: Retained<NSButton>,
}

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<RadioButtonHandler>>> = RefCell::new(HashMap::new());
    static GROUPS: RefCell<HashMap<usize, HashMap<String, RegisteredRadioButton>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn RadioButton(props: &RadioButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__RadioButton", "__appkit_RadioButton"];

    let window = element.context::<WindowContext>().unwrap();
    let window_id = window.ns_window.as_ref() as *const NSWindow as usize;
    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().unwrap();
    let handler = RadioButtonHandler::new(
        mtm,
        RadioButtonHandlerState {
            window_id,
            group: props.group.clone(),
            on_select: props.on_select.clone(),
        },
    );
    let radio = unsafe {
        NSButton::radioButtonWithTitle_target_action(
            &NSString::from_str(&props.title.get()),
            Some(&handler),
            Some(sel!(selected:)),
            mtm,
        )
    };
    let original_font = radio.font().unwrap();
    let original_color = radio.contentTintColor();
    let id = nanoid::nanoid!();

    HANDLERS.with_borrow_mut(|handlers| handlers.insert(id.clone(), handler));
    GROUPS.with_borrow_mut(|groups| {
        groups.entry(window_id).or_default().insert(
            id.clone(),
            RegisteredRadioButton {
                group: props.group.clone(),
                button: radio.clone(),
            },
        );
    });
    element.on_unmount(closure!(
        [id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&id));
            GROUPS.with_borrow_mut(|groups| {
                if let Some(window_group) = groups.get_mut(&window_id) {
                    window_group.remove(&id);
                    if window_group.is_empty() {
                        groups.remove(&window_id);
                    }
                }
            });
        }
    ));

    let content_revision = create_state(0usize);
    native_control::mount(
        element,
        radio.clone().into_super().into_super(),
        style_props.clone(),
        &props.view,
        content_revision.clone().into_readonly(),
    );

    scoped_effect!(
        [
            radio,
            style_props,
            props.title,
            props.enabled,
            props.selected,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            original_font,
            original_color,
            content_revision
        ] || {
            radio.setTitle(&NSString::from_str(&title.get()));
            radio.setEnabled(enabled.get());
            radio.setState(if selected.get() {
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
            radio.setFont(Some(&resolve_font(&original_font, &font_props, mtm)));
            if let Some(color) = font_props.text_color {
                radio.setContentTintColor(Some(&ns_color(color)));
            } else {
                radio.setContentTintColor(original_color.as_deref());
            }
            content_revision.mutate(|revision| *revision = revision.wrapping_add(1));
        }
    );
}

#[derive(Debug)]
struct RadioButtonHandlerState {
    window_id: usize,
    group: PropValue<String>,
    on_select: PropValue<Option<Shared<dyn Fn()>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = RadioButtonHandlerState]
    #[derive(Debug)]
    struct RadioButtonHandler;

    unsafe impl NSObjectProtocol for RadioButtonHandler {}

    impl RadioButtonHandler {
        #[unsafe(method(selected:))]
        fn selected(&self, sender: &NSButton) {
            let group = self.ivars().group.get();
            GROUPS.with_borrow(|groups| {
                if let Some(buttons) = groups.get(&self.ivars().window_id) {
                    for registered in buttons.values() {
                        if registered.group.get() == group {
                            registered.button.setState(NSControlStateValueOff);
                        }
                    }
                }
            });
            sender.setState(NSControlStateValueOn);
            if let Some(callback) = self.ivars().on_select.get() {
                callback();
            }
        }
    }
);

impl RadioButtonHandler {
    fn new(mtm: MainThreadMarker, state: RadioButtonHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
