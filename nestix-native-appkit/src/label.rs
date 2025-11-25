use nestix::{Element, component, effect};
use nestix_native_core::LabelProps;
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSObject, NSPoint, NSSize, NSString};

use crate::ParentContext;

#[component]
pub fn Label(props: &LabelProps, element: &Element) {
    let mtm = MainThreadMarker::new().unwrap();
    let ns_string = NSString::from_str(&props.text.get());
    let label = NSTextField::labelWithString(&ns_string, mtm);

    element.provide_handle(label.as_ref() as *const NSObject);

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

    let parent = element.context::<ParentContext>();
    if let Some(parent) = parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&label);
        }
    }
}
