use nestix::{Element, component, components::ContextProvider, effect, layout};
use nestix_native_core::WindowProps;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

#[derive(Clone)]
pub struct AppkitWindowContext {
    pub window: Retained<NSWindow>,
}

#[component]
pub fn AppkitWindow(props: &WindowProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();

    let masks = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;

    let window = unsafe { NSWindow::new(mtm) };
    window.setStyleMask(masks);
    window.makeKeyAndOrderFront(None);

    element.provide_handle(window.as_ref() as *const NSWindow);

    effect!(window, props.title => || {
        let ns_string = NSString::from_str(&title.get());
        window.setTitle(&ns_string);
    });

    effect!(window, props.width, props.height => || {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width.get(), height.get()));
        window.setFrame_display(frame, true);
    });

    window.center();

    effect!(window, props.view => || {
        if let Some(element) = view.get() {
            if let Some(handle) = element.handle().get() {
                let ns_view = handle.downcast_ref::<*const NSView>().unwrap();
                let ns_view = unsafe { &**ns_view };
                window.setContentView(Some(ns_view));
            }
        }
    });

    layout! {
        ContextProvider<AppkitWindowContext>(
            .value = AppkitWindowContext {
                window
            },
        ) {
            $option(props.view.get())
        }
    }
}
