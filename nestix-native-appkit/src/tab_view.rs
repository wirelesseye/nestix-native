use nestix::{Element, callback, closure, component, components::ContextProvider, effect, layout};
use nestix_native_core::{TabViewItemProps, TabViewProps};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSTabView, NSTabViewItem, NSView};
use objc2_foundation::{NSObject, NSString};

use crate::ParentContext;

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NSTabView::new(mtm);

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

    let ns_object: Retained<NSObject> = unsafe { Retained::cast_unchecked(view.clone()) };

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                ns_object: Some(ns_object),
                add_child: Some(callback!([view] |child: &NSObject| {
                    view.addTabViewItem(child.downcast_ref::<NSTabViewItem>().unwrap());
                }))
            },
            .children = props.children.clone(),
        )
    }
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let id = NSString::from_str(&props.id.get());
    let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), Some(&id)) };

    element.provide_handle(item.as_ref() as *const NSObject);

    let parent = element.context::<ParentContext>();
    if let Some(parent) = &parent {
        if let Some(add_child) = &parent.add_child {
            add_child(&item);
        }
    }

    element.on_destroy(closure!(
        [parent, item] || {
            if let Some(parent) = &parent {
                if let Some(ns_object) = &parent.ns_object {
                    let tab_view = ns_object.downcast_ref::<NSTabView>().unwrap();
                    tab_view.removeTabViewItem(&item);
                }
            }
        }
    ));

    effect!(
        [item, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            item.setLabel(&ns_string);
        }
    );

    let ns_object: Retained<NSObject> = unsafe { Retained::cast_unchecked(item.clone()) };

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                ns_object: Some(ns_object),
                add_child: Some(callback!([item] |child: &NSObject| {
                    let view = child.downcast_ref::<NSView>().unwrap();
                    item.setView(Some(view));
                }))
            },
        ) {
            $option(props.view.get())
        }
    }
}
