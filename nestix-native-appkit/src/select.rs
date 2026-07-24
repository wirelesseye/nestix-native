use std::{cell::RefCell, collections::HashMap};

use nestix::{
    Element, PropValue, Shared, State, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{SelectOptionProps, SelectProps, StyleContext, matched_style};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::{NSMenuItem, NSPopUpButton};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use crate::native_control;

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<SelectHandler>>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct SelectContext {
    popup: Retained<NSPopUpButton>,
    revision: State<usize>,
}

#[component]
pub fn Select(props: &SelectProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Select", "__appkit_Select"];

    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let mtm = MainThreadMarker::new().unwrap();
    let handler = SelectHandler::new(
        mtm,
        SelectHandlerState {
            on_value_change: props.on_value_change.clone(),
        },
    );
    let popup = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
        false,
    );
    unsafe {
        popup.setTarget(Some(&handler));
        popup.setAction(Some(sel!(changed:)));
    }

    let handler_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(handler_id.clone(), handler));
    element.on_unmount(closure!(
        [handler_id] || {
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&handler_id));
        }
    ));

    let revision = create_state(0usize);
    native_control::mount(
        element,
        popup.clone().into_super().into_super().into_super(),
        style_props,
        &props.view,
        revision.clone().into_readonly(),
    );

    scoped_effect!(
        [popup, props.enabled] || {
            popup.setEnabled(enabled.get());
        }
    );
    scoped_effect!(
        [popup, props.value, revision] || {
            let _ = revision.get();
            let desired = value.get();
            let items = popup.itemArray();
            let values = items
                .iter()
                .map(|item| option_value(&item).unwrap_or_default())
                .collect::<Vec<_>>();
            if let Some(index) =
                matching_option_index(values.iter().map(String::as_str), desired.as_deref())
            {
                popup.selectItemAtIndex(index as _);
            } else {
                popup.selectItem(None);
            }
        }
    );

    layout! {
        ContextProvider<SelectContext>(SelectContext { popup, revision }) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn SelectOption(props: &SelectOptionProps, element: &Element) {
    let context = element.context::<SelectContext>().unwrap();
    let mtm = MainThreadMarker::new().unwrap();
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(&props.label.get()),
            None,
            &NSString::new(),
        )
    };

    element.on_place(closure!(
        [context, item] | placement | {
            let menu = context.popup.menu().unwrap();
            if menu.indexOfItem(&item) >= 0 {
                menu.removeItem(&item);
            }
            let index = placement
                .index
                .unwrap_or(menu.numberOfItems() as usize)
                .min(menu.numberOfItems() as usize);
            menu.insertItem_atIndex(&item, index as _);
            context
                .revision
                .mutate(|revision| *revision = revision.wrapping_add(1));
        }
    ));
    element.on_unmount(closure!(
        [context, item] || {
            let menu = context.popup.menu().unwrap();
            if menu.indexOfItem(&item) >= 0 {
                menu.removeItem(&item);
                context
                    .revision
                    .mutate(|revision| *revision = revision.wrapping_add(1));
            }
        }
    ));

    scoped_effect!(
        [item, props.label, props.value, props.enabled] || {
            item.setTitle(&NSString::from_str(&label.get()));
            item.setEnabled(enabled.get());
            let value = NSString::from_str(&value.get());
            unsafe { item.setRepresentedObject(Some(&value)) };
            context
                .revision
                .mutate(|revision| *revision = revision.wrapping_add(1));
        }
    );
}

fn option_value(item: &NSMenuItem) -> Option<String> {
    item.representedObject()
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|value| value.to_string())
}

fn matching_option_index<'a>(
    values: impl Iterator<Item = &'a str>,
    desired: Option<&str>,
) -> Option<usize> {
    let desired = desired?;
    values
        .enumerate()
        .find_map(|(index, value)| (value == desired).then_some(index))
}

#[derive(Debug)]
struct SelectHandlerState {
    on_value_change: PropValue<Option<Shared<dyn Fn(&str)>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SelectHandlerState]
    #[derive(Debug)]
    struct SelectHandler;

    unsafe impl NSObjectProtocol for SelectHandler {}

    impl SelectHandler {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSPopUpButton) {
            if let Some(callback) = self.ivars().on_value_change.get()
                && let Some(item) = sender.selectedItem()
                && let Some(value) = option_value(&item)
            {
                callback(&value);
            }
        }
    }
);

impl SelectHandler {
    fn new(mtm: MainThreadMarker, state: SelectHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use super::matching_option_index;

    #[test]
    fn controlled_selection_resolves_first_matching_value() {
        let values = ["first", "duplicate", "duplicate"];
        assert_eq!(
            matching_option_index(values.into_iter(), Some("duplicate")),
            Some(1)
        );
        assert_eq!(
            matching_option_index(values.into_iter(), Some("missing")),
            None
        );
        assert_eq!(matching_option_index(values.into_iter(), None), None);
    }
}
