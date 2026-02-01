use nestix::{Element, closure, component, effect};
use nestix_native_core::{ExtendsViewProps, LabelProps, Length};
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSObject, NSPoint, NSSize, NSString};

use crate::{ParentContext, WindowContext};

#[component]
pub fn Label(props: &LabelProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();
    let ns_string = NSString::from_str(&props.text.get());
    let label = NSTextField::labelWithString(&ns_string, mtm);

    element.provide_handle(label.as_ref() as *const NSObject);

    effect!(
        [label, window_context.scale_factor, props.x(), props.y()] || {
            let scale_factor = scale_factor.get();
            let x: f64 = match x.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            let y: f64 = match y.get() {
                Length::Auto => 0.0,
                Length::Px(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
            };
            label.setFrameOrigin(NSPoint::new(x, y));
        }
    );

    effect!(
        [
            label,
            window_context.scale_factor,
            props.width(),
            props.height()
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
            label.setFrameSize(NSSize::new(width, height));
        }
    );

    effect!(
        [label, props.text] || {
            let ns_string = NSString::from_str(&text.get());
            label.setStringValue(&ns_string);
        }
    );

    element.on_destroy(closure!(
        [label] || {
            label.removeFromSuperview();
        }
    ));

    let parent = element.context::<ParentContext>();
    if let Some(parent) = parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&label);
        }
    }
}
