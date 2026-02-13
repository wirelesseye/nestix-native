use nestix::{Element, callback, closure, component, components::ContextProvider, effect, layout};
use nestix_native_core::{LinearViewAlignment, LinearViewDirection, LinearViewProps};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSLayoutAttribute, NSStackView, NSUserInterfaceLayoutOrientation, NSView};
use objc2_foundation::NSObject;

use crate::ParentContext;

#[component]
pub fn LinearView(props: &LinearViewProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NSStackView::new(mtm);

    element.provide_handle(view.as_ref() as *const NSObject);

    let parent = element.context::<ParentContext>();
    if let Some(parent) = parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&view);
        }
    }

    element.on_destroy(closure!(
        [view] || {
            view.removeFromSuperview();
        }
    ));

    effect!(
        [view, props.direction] || {
            let orientation = match direction.get() {
                LinearViewDirection::Horizontal => NSUserInterfaceLayoutOrientation::Horizontal,
                LinearViewDirection::Vertical => NSUserInterfaceLayoutOrientation::Vertical,
            };
            view.setOrientation(orientation);
        }
    );

    effect!(
        [view, props.alignment, props.direction] || {
            let alignment = match alignment.get() {
                LinearViewAlignment::Unset => match direction.get() {
                    LinearViewDirection::Horizontal => NSLayoutAttribute::Top,
                    LinearViewDirection::Vertical => NSLayoutAttribute::Leading,
                },
                LinearViewAlignment::Start => match direction.get() {
                    LinearViewDirection::Horizontal => NSLayoutAttribute::Top,
                    LinearViewDirection::Vertical => NSLayoutAttribute::Leading,
                },
                LinearViewAlignment::End => match direction.get() {
                    LinearViewDirection::Horizontal => NSLayoutAttribute::Bottom,
                    LinearViewDirection::Vertical => NSLayoutAttribute::Trailing,
                },
                LinearViewAlignment::Center => match direction.get() {
                    LinearViewDirection::Horizontal => NSLayoutAttribute::CenterY,
                    LinearViewDirection::Vertical => NSLayoutAttribute::CenterX,
                },
            };
            view.setAlignment(alignment);
        }
    );

    let ns_object: Retained<NSObject> = unsafe { Retained::cast_unchecked(view.clone()) };

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                ns_object: Some(ns_object),
                add_child: Some(callback!([view] |child: &NSObject| {
                    view.addArrangedSubview(child.downcast_ref::<NSView>().unwrap());
                }))
            },
            .children = props.children.clone(),
        )
    }
}
