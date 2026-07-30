use std::{cell::Cell, rc::Rc};

use gtk4::prelude::*;
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    AnimatedStyle, SliderProps, StyleContext, matched_style, resolved_view_style,
};

use crate::{WindowContext, layout::mount_leaf};

#[component]
/// Renders a native horizontal GTK slider.
pub fn Slider(props: &SliderProps, element: &Element) {
    require_visual_mount!(element, Slider);
    const DEFAULT_CLASSES: [&str; 2] = ["__Slider", "__gtk4_Slider"];

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

    let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
    slider.set_draw_value(false);
    let updating = Rc::new(Cell::new(false));
    slider.connect_value_changed(closure!(
        [props.on_value_change, updating] | slider | {
            if !updating.get()
                && let Some(callback) = on_value_change.get()
            {
                callback(slider.value());
            }
        }
    ));
    scoped_effect!([slider, props.enabled] || slider.set_sensitive(enabled.get()));
    scoped_effect!(
        [slider, props.value, props.minimum, props.maximum, updating] || {
            let (minimum, maximum) = valid_range(minimum.get(), maximum.get());
            let value = if value.get().is_finite() {
                value.get().clamp(minimum, maximum)
            } else {
                minimum
            };
            updating.set(true);
            slider.set_range(minimum, maximum);
            slider.set_value(value);
            updating.set(false);
        }
    );

    mount_leaf(
        element,
        slider.upcast_ref(),
        style.into_readonly(),
        &props.view,
        create_state(0usize).0.into_readonly(),
    );
}

fn valid_range(minimum: f64, maximum: f64) -> (f64, f64) {
    if minimum.is_finite() && maximum.is_finite() && maximum > minimum {
        (minimum, maximum)
    } else {
        (0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ranges_use_the_default() {
        assert_eq!(valid_range(5.0, 5.0), (0.0, 100.0));
        assert_eq!(valid_range(f64::NAN, 5.0), (0.0, 100.0));
        assert_eq!(valid_range(-5.0, 5.0), (-5.0, 5.0));
    }
}
