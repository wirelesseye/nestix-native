use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use gtk4::prelude::*;
use nestix::{
    Element, State, closure, component, components::ContextProvider, create_state, layout,
    scoped_effect,
};
use nestix_native_core::{
    AnimatedStyle, SelectOptionProps, SelectProps, StyleContext, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf};

static NEXT_OPTION_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone)]
struct SelectContext {
    combo: gtk4::ComboBoxText,
    options: Rc<RefCell<Vec<OptionEntry>>>,
    revision: State<usize>,
    updating: Rc<Cell<bool>>,
}

#[derive(Clone)]
struct OptionEntry {
    id: usize,
    label: String,
    value: String,
    enabled: bool,
}

#[component]
/// Renders a native GTK selection control.
pub fn Select(props: &SelectProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Select", "__gtk4_Select"];

    let window_context = element.context::<WindowContext>().unwrap();
    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_style = resolved_view_style(matched, &props.view);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        target_style.get(),
    ));
    let style = animated_style.value();
    scoped_effect!(
        [animated_style, target_style, window_context.scale_factor] || {
            animated_style.set_target(target_style.get(), scale_factor.get());
        }
    );

    let combo = gtk4::ComboBoxText::new();
    let options = Rc::new(RefCell::new(Vec::<OptionEntry>::new()));
    let revision = create_state(0usize);
    let updating = Rc::new(Cell::new(false));
    combo.connect_changed(closure!(
        [props.value, props.on_value_change, options, updating] | combo | {
            if updating.get() {
                return;
            }
            let selected = combo.active_id().map(|value| value.to_string());
            let enabled = selected.as_deref().is_some_and(|selected| {
                options
                    .borrow()
                    .iter()
                    .find(|option| option.value == selected)
                    .is_some_and(|option| option.enabled)
            });
            if enabled {
                if let Some(callback) = on_value_change.get() {
                    callback(selected.as_deref().unwrap());
                }
            } else {
                set_active_value(combo, value.get().as_deref(), &updating);
            }
        }
    ));

    scoped_effect!([combo, props.enabled] || combo.set_sensitive(enabled.get()));
    scoped_effect!(
        [combo, props.value, revision, updating] || {
            let _ = revision.get();
            set_active_value(&combo, value.get().as_deref(), &updating);
        }
    );

    mount_leaf(
        element,
        combo.upcast_ref(),
        style.into_readonly(),
        &props.view,
        revision.clone().into_readonly(),
    );

    layout! {
        ContextProvider<SelectContext>(SelectContext { combo, options, revision, updating }) {
            $(props.children.clone())
        }
    }
}

#[component]
/// Registers an option with its containing [`Select`].
pub fn SelectOption(props: &SelectOptionProps, element: &Element) {
    let context = element.context::<SelectContext>().unwrap();
    let id = NEXT_OPTION_ID.fetch_add(1, Ordering::Relaxed);
    let initial = OptionEntry {
        id,
        label: props.label.get(),
        value: props.value.get(),
        enabled: props.enabled.get(),
    };

    element.on_place(closure!(
        [context] | placement | {
            let mut options = context.options.borrow_mut();
            let option = options
                .iter()
                .position(|option| option.id == id)
                .map(|index| options.remove(index))
                .unwrap_or_else(|| initial.clone());
            let index = placement.index.unwrap_or(options.len()).min(options.len());
            options.insert(index, option);
            drop(options);
            rebuild(&context);
        }
    ));
    element.on_unmount(closure!(
        [context] || {
            context
                .options
                .borrow_mut()
                .retain(|option| option.id != id);
            rebuild(&context);
        }
    ));
    scoped_effect!(
        [context, props.label, props.value, props.enabled] || {
            let changed = if let Some(option) = context
                .options
                .borrow_mut()
                .iter_mut()
                .find(|option| option.id == id)
            {
                option.label = label.get();
                option.value = value.get();
                option.enabled = enabled.get();
                true
            } else {
                false
            };
            if changed {
                rebuild(&context);
            }
        }
    );
}

fn rebuild(context: &SelectContext) {
    context.updating.set(true);
    context.combo.remove_all();
    for option in context.options.borrow().iter() {
        context.combo.append(Some(&option.value), &option.label);
    }
    context.updating.set(false);
    context
        .revision
        .mutate(|revision| *revision = revision.wrapping_add(1));
}

fn set_active_value(combo: &gtk4::ComboBoxText, value: Option<&str>, updating: &Cell<bool>) {
    updating.set(true);
    match value {
        Some(value) => {
            combo.set_active_id(Some(value));
        }
        None => combo.set_active(None),
    }
    updating.set(false);
}
