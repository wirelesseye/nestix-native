use std::{cell::Cell, rc::Rc};

use gtk4::prelude::*;
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{InputProps, StyleContext, matched_style};

use crate::layout::mount_leaf;

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__gtk4_Input"];

    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let input = gtk4::Entry::new();
    input.set_text(&props.value.get());
    let content_revision = create_state(0usize);
    let updating_value = Rc::new(Cell::new(false));

    input.connect_changed(closure!(
        [props.on_text_change, updating_value] | input | {
            if !updating_value.get()
                && let Some(on_text_change) = on_text_change.get()
            {
                on_text_change(input.text().as_str());
            }
        }
    ));

    scoped_effect!(
        element,
        [input, props.value, content_revision, updating_value] || {
            let value = value.get();
            if input.text().as_str() != value {
                updating_value.set(true);
                input.set_text(&value);
                updating_value.set(false);
            }
            content_revision.mutate(|revision| *revision += 1);
        }
    );

    mount_leaf(
        element,
        input.upcast_ref(),
        style_props,
        &props.view,
        content_revision.into_readonly(),
    );
}
