use std::{cell::Cell, rc::Rc};

use gtk4::{Orientation, prelude::*};
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, InputProps, StyleContext, dpi::LogicalSize, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf_with_intrinsic_size};

#[component]
pub fn Input(props: &InputProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Input", "__gtk4_Input"];

    let style_context = element.context::<StyleContext>();
    let window_context = element.context::<WindowContext>().unwrap();
    let matched_style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let target_style = resolved_view_style(matched_style_props, &props.view);
    let animated_style = Rc::new(AnimatedStyle::new(
        window_context.animation.clone(),
        target_style.get(),
    ));
    let style_props = animated_style.value();
    scoped_effect!(
        [animated_style, target_style, window_context.scale_factor] || {
            animated_style.set_target(target_style.get(), scale_factor.get());
        }
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

    let (_, natural_width, _, _) = input.measure(Orientation::Horizontal, -1);
    let (_, natural_height, _, _) = input.measure(Orientation::Vertical, natural_width);
    mount_leaf_with_intrinsic_size(
        element,
        input.upcast_ref(),
        style_props.into_readonly(),
        &props.view,
        content_revision.into_readonly(),
        LogicalSize::new(natural_width.max(0) as f32, natural_height.max(0) as f32),
    );
}
