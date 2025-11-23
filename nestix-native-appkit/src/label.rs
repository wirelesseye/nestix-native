use nestix::{component, derive_props, effect, provide_handle, use_context};
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::stack_view::ParentViewContext;

#[derive_props]
pub struct AppkitLabelProps {
    text: String,

    #[props(default = 0.0)]
    x: f64,
    #[props(default = 0.0)]
    y: f64,

    #[props(default = 100.0)]
    width: f64,
    #[props(default = 24.0)]
    height: f64,
}

#[component]
pub fn AppkitLabel(props: &AppkitLabelProps) {
    let mtm = MainThreadMarker::new().unwrap();
    let rect = NSRect::new(
        NSPoint::new(props.x.get(), props.y.get()),
        NSSize::new(props.width.get(), props.height.get()),
    );
    let label = NSTextField::initWithFrame(mtm.alloc(), rect);
    label.setBezeled(false);
    label.setDrawsBackground(false);
    label.setEditable(false);
    label.setSelectable(false);

    provide_handle(label.as_ref() as *const NSTextField);

    effect!(label, props.x, props.y => || {
        label.setFrameOrigin(NSPoint::new(x.get(), y.get()));
    });

    effect!(label, props.width, props.height => || {
        label.setFrameSize(NSSize::new(width.get(), height.get()));
    });

    effect!(label, props.text => || {
        let ns_string = NSString::from_str(&text.get());
        label.setStringValue(&ns_string);
    });

    let parent = use_context::<ParentViewContext>();
    if let Some(parent) = parent {
        (parent.add_child)(&label);
    }
}
