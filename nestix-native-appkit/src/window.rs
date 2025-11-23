use nestix::{
    Element, component, components::ContextProvider, derive_props, effect, layout, provide_handle,
};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

#[derive_props]
pub struct AppkitWindowProps {
    view: Option<Element>,

    #[props(default)]
    title: String,

    #[props(default = 800.0)]
    width: f64,
    #[props(default = 600.0)]
    height: f64,
}

#[derive(Clone)]
pub struct AppkitWindowContext {
    pub window: Retained<NSWindow>,
}

#[component]
pub fn AppkitWindow(props: &AppkitWindowProps) -> Element {
    let mtm = MainThreadMarker::new().unwrap();

    let masks = NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::Titled;

    let window = unsafe { NSWindow::new(mtm) };
    window.setStyleMask(masks);
    window.makeKeyAndOrderFront(None);

    provide_handle(window.as_ref() as *const NSWindow);

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
            #![clone(props.view)]
            yield $option(view.get())
        }
    }
}
