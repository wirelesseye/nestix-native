use nestix::{
    Element, callback, component, components::ContextProvider, effect, layout,
};
use nestix_native_core::{ListViewDirection, ListViewProps};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSStackView, NSUserInterfaceLayoutOrientation, NSView};

use crate::ParentViewContext;

#[component]
pub fn AppkitListView(props: &ListViewProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NSStackView::new(mtm);

    element.provide_handle(view.as_ref() as *const NSView);

    let parent = element.context::<ParentViewContext>();
    if let Some(parent) = parent {
        (parent.add_child)(&view);
    }

    effect!(view, props.direction => || {
        let orientation = match direction.get() {
            ListViewDirection::Horizontal => NSUserInterfaceLayoutOrientation::Horizontal,
            ListViewDirection::Vertical => NSUserInterfaceLayoutOrientation::Vertical,
        };
        view.setOrientation(orientation);
    });

    layout! {
        ContextProvider<ParentViewContext>(
            .value = ParentViewContext {
                add_child: callback!(view => |subview: &NSView| {
                    view.addArrangedSubview(subview);
                })
            },
            .children = props.children.clone(),
        )
    }
}
