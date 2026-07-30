use std::{cell::Cell, rc::Rc};

use gtk4::prelude::*;
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, CheckboxProps, StyleContext, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf};

#[component]
/// Renders a native GTK checkbox.
pub fn Checkbox(props: &CheckboxProps, element: &Element) {
    require_visual_mount!(element, Checkbox);
    const DEFAULT_CLASSES: [&str; 2] = ["__Checkbox", "__gtk4_Checkbox"];

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

    let checkbox = gtk4::CheckButton::with_label(&props.title.get());
    let updating = Rc::new(Cell::new(false));
    let (content_revision, set_content_revision) = create_state(0usize);
    checkbox.connect_toggled(closure!(
        [props.on_checked_change, updating] | checkbox | {
            if !updating.get()
                && let Some(callback) = on_checked_change.get()
            {
                callback(checkbox.is_active());
            }
        }
    ));

    scoped_effect!([checkbox, props.enabled] || checkbox.set_sensitive(enabled.get()));
    scoped_effect!(
        [checkbox, props.title, content_revision] || {
            checkbox.set_label(Some(&title.get()));
            set_content_revision.mutate(|revision| *revision = revision.wrapping_add(1));
        }
    );
    scoped_effect!(
        [checkbox, props.checked, updating] || {
            let checked = checked.get();
            if checkbox.is_active() != checked {
                updating.set(true);
                checkbox.set_active(checked);
                updating.set(false);
            }
        }
    );

    mount_leaf(
        element,
        checkbox.upcast_ref(),
        style.into_readonly(),
        &props.view,
        content_revision.into_readonly(),
    );
}
