use std::{cell::Cell, rc::Rc};

use gtk4::prelude::*;
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, StyleContext, SwitchProps, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf};

#[component]
/// Renders a native GTK switch.
pub fn Switch(props: &SwitchProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Switch", "__gtk4_Switch"];

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

    let switch = gtk4::Switch::new();
    let updating = Rc::new(Cell::new(false));
    switch.connect_active_notify(closure!(
        [props.on_checked_change, updating] | switch | {
            if !updating.get()
                && let Some(callback) = on_checked_change.get()
            {
                callback(switch.is_active());
            }
        }
    ));
    scoped_effect!([switch, props.enabled] || switch.set_sensitive(enabled.get()));
    scoped_effect!(
        [switch, props.checked, updating] || {
            let checked = checked.get();
            if switch.is_active() != checked {
                updating.set(true);
                switch.set_active(checked);
                updating.set(false);
            }
        }
    );

    mount_leaf(
        element,
        switch.upcast_ref(),
        style.into_readonly(),
        &props.view,
        create_state(0usize).into_readonly(),
    );
}
