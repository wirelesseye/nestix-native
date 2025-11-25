use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, Shared, closure, component, effect, prop::PropValue};
use nestix_native_core::ButtonProps;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::NSButton;
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSSize, NSString};

use crate::ParentContext;

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<ButtonHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
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
    element.on_destroy(closure!(button_id => || {
        HANDLERS.with_borrow_mut(|handlers| handlers.remove(&button_id));
    }));
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(button_id, handler));

    element.provide_handle(button.as_ref() as *const NSObject);

    effect!(button, props.x, props.y => || {
        button.setFrameOrigin(NSPoint::new(x.get(), y.get()));
    });

    effect!(button, props.width, props.height => || {
        button.setFrameSize(NSSize::new(width.get(), height.get()));
    });

    effect!(button, props.title => || {
        let ns_string = NSString::from_str(&title.get());
        button.setTitle(&ns_string);
    });

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
