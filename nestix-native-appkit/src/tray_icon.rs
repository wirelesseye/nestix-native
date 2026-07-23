use std::{
    cell::RefCell,
    path::PathBuf,
    rc::{Rc, Weak},
};

use block2::RcBlock;
use nestix::{
    Element, PropValue, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{ImageSource, TrayIconError, TrayIconEvent, TrayIconProps};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained, sel,
};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSEventMask, NSEventModifierFlags, NSEventType, NSImage,
    NSImageScaling, NSMenu, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSObject, NSObjectProtocol, NSOperationQueue, NSSize, NSString};

use crate::menu::TrayMenuContext;

struct TrayIconState {
    status_bar: Retained<NSStatusBar>,
    status_item: Option<Retained<NSStatusItem>>,
    menu: State<Option<Retained<NSMenu>>>,
    menu_presentation_pending: bool,
}

impl TrayIconState {
    fn remove_item(&mut self) {
        self.menu_presentation_pending = false;
        if let Some(item) = self.status_item.take() {
            item.setMenu(None);
            self.status_bar.removeStatusItem(&item);
        }
    }
}

fn show_menu(state: &Weak<RefCell<TrayIconState>>) -> Result<(), TrayIconError> {
    let state = state.upgrade().ok_or(TrayIconError::NotMounted)?;
    let (item, menu) = {
        let mut state = state.borrow_mut();
        if state.menu_presentation_pending {
            return Err(TrayIconError::PresentationFailed);
        }
        let item = state
            .status_item
            .as_ref()
            .ok_or(TrayIconError::NotMounted)?
            .clone();
        let menu = state.menu.get().ok_or(TrayIconError::MenuUnavailable)?;
        state.menu_presentation_pending = true;
        (item, menu)
    };

    let weak_state = Rc::downgrade(&state);
    let presentation = RcBlock::new(move || {
        let should_present = weak_state.upgrade().is_some_and(|state| {
            let state = state.borrow();
            state.menu_presentation_pending
                && state.status_item.as_ref().is_some_and(|current| {
                    std::ptr::eq::<NSStatusItem>(current.as_ref(), item.as_ref())
                })
        });
        if !should_present {
            return;
        }

        let mtm = MainThreadMarker::new().unwrap();
        let Some(button) = item.button(mtm) else {
            if let Some(state) = weak_state.upgrade() {
                state.borrow_mut().menu_presentation_pending = false;
            }
            return;
        };

        item.setMenu(Some(&menu));
        unsafe { button.performClick(None) };
        item.setMenu(None);

        if let Some(state) = weak_state.upgrade() {
            state.borrow_mut().menu_presentation_pending = false;
        }
    });
    unsafe {
        NSOperationQueue::mainQueue().addOperationWithBlock(&presentation);
    }
    Ok(())
}

fn event_for(state: &Weak<RefCell<TrayIconState>>) -> TrayIconEvent {
    let state = state.clone();
    TrayIconEvent::new(callback!(move || show_menu(&state)))
}

struct TrayIconHandlerState {
    tray: Weak<RefCell<TrayIconState>>,
    on_activate: PropValue<Option<nestix::Shared<dyn Fn(TrayIconEvent)>>>,
    on_secondary: PropValue<Option<nestix::Shared<dyn Fn(TrayIconEvent)>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixTrayIconHandler"]
    #[ivars = TrayIconHandlerState]
    struct TrayIconHandler;

    unsafe impl NSObjectProtocol for TrayIconHandler {}

    impl TrayIconHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, _sender: &NSObject) {
            let mtm = MainThreadMarker::new().unwrap();
            let application = NSApplication::sharedApplication(mtm);
            let secondary = application.currentEvent().is_some_and(|event| {
                matches!(
                    event.r#type(),
                    NSEventType::RightMouseDown
                        | NSEventType::RightMouseUp
                        | NSEventType::OtherMouseDown
                        | NSEventType::OtherMouseUp
                ) || event
                    .modifierFlags()
                    .contains(NSEventModifierFlags::Control)
            });
            let event = event_for(&self.ivars().tray);
            let callback = if secondary {
                self.ivars().on_secondary.get()
            } else {
                self.ivars().on_activate.get()
            };
            if let Some(callback) = callback {
                callback(event);
            }
        }
    }
);

impl TrayIconHandler {
    fn new(mtm: MainThreadMarker, state: TrayIconHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

fn load_image(source: ImageSource) -> Option<Retained<NSImage>> {
    let image = match source {
        ImageSource::File(path) => {
            let path = NSString::from_str(&path_to_string(path));
            NSImage::initWithContentsOfFile(NSImage::alloc(), &path)
        }
        ImageSource::Bytes(bytes) => {
            let data = NSData::with_bytes(&bytes);
            NSImage::initWithData(NSImage::alloc(), &data)
        }
    }?;
    let size = image.size();
    let icon_height = 18.0;
    let icon_width = if size.width.is_finite()
        && size.height.is_finite()
        && size.width > 0.0
        && size.height > 0.0
    {
        size.width * icon_height / size.height
    } else {
        icon_height
    };
    image.setSize(NSSize::new(icon_width, icon_height));
    Some(image)
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Application".to_string())
}

#[component]
pub fn TrayIcon(props: &TrayIconProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let menu = create_state(None::<Retained<NSMenu>>);
    let state = Rc::new(RefCell::new(TrayIconState {
        status_bar: NSStatusBar::systemStatusBar(),
        status_item: None,
        menu: menu.clone(),
        menu_presentation_pending: false,
    }));
    let handler = TrayIconHandler::new(
        mtm,
        TrayIconHandlerState {
            tray: Rc::downgrade(&state),
            on_activate: props.on_activate.clone(),
            on_secondary: props.on_secondary.clone(),
        },
    );

    scoped_effect!(
        element,
        [
            state,
            handler,
            props.visible,
            props.icon,
            props.tooltip,
            props.template
        ] || {
            if !visible.get() {
                state.borrow_mut().remove_item();
                return;
            }

            let image = load_image(icon.get());
            if image.is_none() && state.borrow().status_item.is_none() {
                eprintln!("nestix-native: could not decode tray icon image");
                return;
            }

            if state.borrow().status_item.is_none() {
                let item = state
                    .borrow()
                    .status_bar
                    .statusItemWithLength(NSVariableStatusItemLength);
                if let Some(button) = item.button(mtm) {
                    unsafe {
                        button.setTarget(Some(handler.as_ref()));
                        button.setAction(Some(sel!(activate:)));
                    }
                    button.sendActionOn(
                        NSEventMask::LeftMouseUp
                            | NSEventMask::RightMouseUp
                            | NSEventMask::OtherMouseUp,
                    );
                    button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                }
                state.borrow_mut().status_item = Some(item);
            }

            let state = state.borrow();
            let item = state.status_item.as_ref().unwrap();
            if let Some(button) = item.button(mtm) {
                if let Some(image) = image {
                    image.setTemplate(template.get());
                    button.setImage(Some(&image));
                }
                let tooltip = tooltip.get();
                let native_tooltip = tooltip.as_ref().map(|value| NSString::from_str(value));
                button.setToolTip(native_tooltip.as_deref());
                let accessibility_label = NSString::from_str(&tooltip.unwrap_or_else(process_name));
                button.setAccessibilityLabel(Some(&accessibility_label));
            }
        }
    );

    element.on_unmount(closure!([state] || state.borrow_mut().remove_item()));

    layout! {
        ContextProvider<TrayMenuContext>(TrayMenuContext { menu }) {
            $(props.menu.clone().map(|menu| nestix::Layout::from(menu.clone())))
        }
    }
}
