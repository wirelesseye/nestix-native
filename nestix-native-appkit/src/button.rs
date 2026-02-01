use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, Shared, closure, component, effect, prop::PropValue};
use nestix_native_core::{ButtonProps, ExtendsViewProps, Length};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::NSButton;
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSSize, NSString};

use crate::{ParentContext, WindowContext};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<ButtonHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();

    let title = NSString::from_str(&props.title.get());
    let handler = ButtonHandler::new(
        mtm,
        ButtonHandlerState {
            on_click: props.on_click.clone(),
        },
    );

    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&title, Some(&handler), Some(sel!(clicked)), mtm)
    };

    let button_id = nanoid::nanoid!();
    element.on_destroy(closure!(
        [button_id, button] || {
            button.removeFromSuperview();
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&button_id));
        }
    ));
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(button_id, handler));

    element.provide_handle(button.as_ref() as *const NSObject);

    effect!(
        [button, window_context.scale_factor, props.x(), props.y()] || {
            let scale_factor = scale_factor.get();
            let x: f64 = match x.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            let y: f64 = match y.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            button.setFrameOrigin(NSPoint::new(x, y));
        }
    );

    effect!(
        [
            button,
            window_context.scale_factor,
            props.width(),
            props.height(),
        ] || {
            let scale_factor = scale_factor.get();
            let width: f64 = match width.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            let height: f64 = match height.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            button.setFrameSize(NSSize::new(width, height));
        }
    );

    effect!(
        [button, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            button.setTitle(&ns_string);
        }
    );

    let parent = element.context::<ParentContext>();
    if let Some(parent) = parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&button);
        }
    }
}

#[derive(Debug)]
struct ButtonHandlerState {
    on_click: PropValue<Option<Shared<dyn Fn()>>>,
}

define_class! {
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonHandlerState]
    #[derive(Debug)]
    struct ButtonHandler;

    unsafe impl NSObjectProtocol for ButtonHandler {}

    impl ButtonHandler {
        #[unsafe(method(clicked))]
        fn clicked(&self) {
            if let Some(on_click) = self.ivars().on_click.get() {
                on_click();
            }
        }
    }
}

impl ButtonHandler {
    fn new(mtm: MainThreadMarker, state: ButtonHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
