use nestix::{
    Element, callback, component, components::ContextProvider, derive_props, effect, layout, provide_handle, use_context
};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSStackView, NSUserInterfaceLayoutOrientation, NSView};

use crate::stack_view::ParentViewContext;

#[derive(Clone, Copy)]
pub enum ListViewDirection {
    Horizontal,
    Vertical,
}

#[derive_props]
pub struct AppkitListViewProps {
    #[props(default = ListViewDirection::Vertical)]
    direction: ListViewDirection,
    children: Option<Vec<Element>>,
}

#[component]
pub fn AppkitListView(props: &AppkitListViewProps) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NSStackView::new(mtm);

    provide_handle(view.as_ref() as *const NSView);

    let parent = use_context::<ParentViewContext>();
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
