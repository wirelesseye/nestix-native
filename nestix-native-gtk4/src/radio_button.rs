use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use gtk4::prelude::*;
use nestix::{Element, PropValue, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, RadioButtonProps, StyleContext, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf};

static NEXT_RADIO_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) struct RegisteredRadioButton {
    id: usize,
    group: PropValue<String>,
    button: gtk4::CheckButton,
}

#[component]
/// Renders a native GTK radio button.
pub fn RadioButton(props: &RadioButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__RadioButton", "__gtk4_RadioButton"];

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

    let radio = gtk4::CheckButton::with_label(&props.title.get());
    radio.add_css_class("radio");
    let id = NEXT_RADIO_ID.fetch_add(1, Ordering::Relaxed);
    window_context
        .radio_buttons
        .borrow_mut()
        .push(RegisteredRadioButton {
            id,
            group: props.group.clone(),
            button: radio.clone(),
        });
    element.on_unmount(closure!(
        [window_context] || {
            window_context
                .radio_buttons
                .borrow_mut()
                .retain(|entry| entry.id != id);
        }
    ));

    let updating = Rc::new(Cell::new(false));
    let content_revision = create_state(0usize);
    radio.connect_toggled(closure!(
        [props.on_select, updating] | radio | {
            if radio.is_active()
                && !updating.get()
                && let Some(callback) = on_select.get()
            {
                callback();
            }
        }
    ));

    scoped_effect!([radio, props.enabled] || radio.set_sensitive(enabled.get()));
    scoped_effect!(
        [radio, props.title, content_revision] || {
            radio.set_label(Some(&title.get()));
            content_revision.mutate(|revision| *revision = revision.wrapping_add(1));
        }
    );
    scoped_effect!(
        [radio, props.group, window_context] || {
            let group = group.get();
            let buttons = window_context.radio_buttons.borrow();
            let peer = buttons
                .iter()
                .find(|entry| entry.id != id && entry.group.get() == group)
                .map(|entry| &entry.button);
            radio.set_group(peer);
        }
    );
    scoped_effect!(
        [radio, props.selected, updating] || {
            let selected = selected.get();
            if radio.is_active() != selected {
                updating.set(true);
                radio.set_active(selected);
                updating.set(false);
            }
        }
    );

    mount_leaf(
        element,
        radio.upcast_ref(),
        style.into_readonly(),
        &props.view,
        content_revision.into_readonly(),
    );
}
