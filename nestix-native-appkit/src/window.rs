use nestix::{Element, callback, component, components::ContextProvider, effect, layout};
use nestix_native_core::WindowProps;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString};

use crate::ParentContext;

#[derive(Clone)]
pub struct WindowContext {
    pub window: Retained<NSWindow>,
}

#[component]
pub fn Window(props: &WindowProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();

    let masks = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;

    let window = unsafe { NSWindow::new(mtm) };
    window.setStyleMask(masks);
    window.makeKeyAndOrderFront(None);

    element.provide_handle(window.as_ref() as *const NSObject);

    effect!(window, props.title => || {
        let ns_string = NSString::from_str(&title.get());
        window.setTitle(&ns_string);
    });

    effect!(window, props.width, props.height => || {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width.get(), height.get()));
        window.setFrame_display(frame, true);
    });

    window.center();

    let ns_object: Retained<NSObject> = unsafe {
        Retained::cast_unchecked(window.clone())
    };

    layout! {
        ContextProvider<WindowContext>(
            .value = WindowContext {
                window: window.clone(),
            },
        ) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    ns_object: Some(ns_object),
                    add_child: Some(callback!(window => |child: &NSObject| {
                        let view = child.downcast_ref::<NSView>().unwrap();
                        window.setContentView(Some(view));
                    }))
                }
            ) {
                $option(props.view.get())
            }
        }
    }
}
