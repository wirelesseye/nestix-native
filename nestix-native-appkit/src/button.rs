use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, scoped_effect};
use nestix_native_core::{
    Appearance, ButtonProps, Dimension, Rect, StyleContext, TreeContext, matched_style,
    resolve_font_props, style_align_self, style_appearance, style_dimension, style_flex_basis,
    style_flex_grow, style_flex_shrink, style_margin, style_padding_with_default,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::NSButton;
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{ParentContext, native_child_index},
    font::{ns_color, resolve_font},
};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<ButtonHandler>>> = RefCell::new(HashMap::new());
}

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__appkit_Button"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );

    let mtm = MainThreadMarker::new().unwrap();
    let title = NSString::from_str(&props.title.get());
    let handler = ButtonHandler::new(
        mtm,
        ButtonHandlerState {
            on_click: props.on_click.clone(),
        },
    );

    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&title, Some(&handler), Some(sel!(clicked)), mtm)
    };
    let original_font = button.font().unwrap();
    let original_color = button.contentTintColor();
    element.provide_handle(button.as_ref() as *const NSObject);

    let button_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(button_id.clone(), handler));

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [button, element, parent_context] | placement | {
            if placement.index.is_some()
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&button, Some(node_id), native_child_index(&element));
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&button, Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [button, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&button, Some(node_id));
            }
            HANDLERS.with_borrow_mut(|handlers| handlers.remove(&button_id));
        }
    ));

    scoped_effect!(
        element,
        [button, props.disabled] || {
            button.setEnabled(!disabled.get());
        }
    );

    scoped_effect!(
        element,
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            style_props,
            button,
            props.view.width,
            props.view.height,
            props.container.padding(),
            props.appearance,
            props.title,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            original_font,
            original_color,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let padding =
                style_padding_with_default(style_props.as_ref(), padding.get(), Dimension::Auto);
            let appearance = style_appearance(style_props.as_ref(), appearance.get());
            let native_appearance = uses_native_appearance(appearance, padding);
            button.setBordered(native_appearance);
            let ns_string = NSString::from_str(&title.get());
            button.setTitle(&ns_string);
            let font_props = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let font = resolve_font(&original_font, &font_props, mtm);
            button.setFont(Some(&font));
            if let Some(color) = font_props.text_color {
                button.setContentTintColor(Some(&ns_color(color)));
            } else {
                button.setContentTintColor(original_color.as_deref());
            }
            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            );

            let intrinsic_size =
                (width.is_auto() || height.is_auto()).then(|| button.intrinsicContentSize());
            let rendered_padding = if native_appearance {
                Rect {
                    top: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                    right: 0.0,
                }
            } else {
                logical_padding(padding, scale_factor)
            };
            let width = match width {
                Dimension::Auto => {
                    intrinsic_size.unwrap().width as f32
                        + rendered_padding.left
                        + rendered_padding.right
                }
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height {
                Dimension::Auto => {
                    intrinsic_size.unwrap().height as f32
                        + rendered_padding.top
                        + rendered_padding.bottom
                }
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    ..prev
                });
            }

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
                style.top
            });

            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, button] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let alignment_rect = NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                );
                button.setFrame(button.frameForAlignmentRect(alignment_rect));
            }
        }
    );
}

fn uses_native_appearance(appearance: Appearance, padding: Rect<Dimension>) -> bool {
    match appearance {
        Appearance::Native => true,
        Appearance::None => false,
        Appearance::Auto => [padding.top, padding.bottom, padding.left, padding.right]
            .into_iter()
            .all(|dimension| dimension == Dimension::Auto),
    }
}

fn logical_padding(padding: Rect<Dimension>, scale_factor: f64) -> Rect<f32> {
    fn logical(dimension: Dimension, scale_factor: f64) -> f32 {
        match dimension {
            Dimension::Auto => 0.0,
            Dimension::Length(value) => value.to_logical::<f32>(scale_factor).0,
        }
    }

    Rect {
        top: logical(padding.top, scale_factor),
        bottom: logical(padding.bottom, scale_factor),
        left: logical(padding.left, scale_factor),
        right: logical(padding.right, scale_factor),
    }
}

#[derive(Debug)]
struct ButtonHandlerState {
    on_click: PropValue<Option<Shared<dyn Fn()>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonHandlerState]
    #[derive(Debug)]
    struct ButtonHandler;

    unsafe impl NSObjectProtocol for ButtonHandler {}

    impl ButtonHandler {
        #[unsafe(method(clicked))]
        fn clicked(&self) {
            if let Some(on_click) = self.ivars().on_click.get() {
                on_click();
            }
        }
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use nestix_native_core::dpi::PhysicalUnit;

    fn padding(value: Dimension) -> Rect<Dimension> {
        Rect {
            top: value,
            bottom: value,
            left: value,
            right: value,
        }
    }

    #[test]
    fn native_and_none_force_their_requested_appearance() {
        assert!(uses_native_appearance(
            Appearance::Native,
            padding(Dimension::from(8))
        ));
        assert!(!uses_native_appearance(
            Appearance::None,
            padding(Dimension::Auto)
        ));
    }

    #[test]
    fn auto_uses_native_appearance_only_for_unspecified_padding() {
        assert!(uses_native_appearance(
            Appearance::Auto,
            padding(Dimension::Auto)
        ));

        for value in [
            Dimension::from(8),
            Dimension::from(0),
            Dimension::from(-8),
            Dimension::Length(PhysicalUnit::new(8).into()),
        ] {
            assert!(!uses_native_appearance(
                Appearance::Auto,
                Rect {
                    top: Dimension::Auto,
                    bottom: Dimension::Auto,
                    left: value,
                    right: Dimension::Auto,
                }
            ));
        }
    }

    #[test]
    fn logical_padding_maps_auto_to_zero_and_scales_physical_values() {
        let padding = logical_padding(
            Rect {
                top: Dimension::Auto,
                bottom: Dimension::from(-2),
                left: Dimension::Length(PhysicalUnit::new(8).into()),
                right: Dimension::from(3),
            },
            2.0,
        );

        assert_eq!(padding.top, 0.0);
        assert_eq!(padding.bottom, -2.0);
        assert_eq!(padding.left, 4.0);
        assert_eq!(padding.right, 3.0);
    }
}

impl ButtonHandler {
    fn new(mtm: MainThreadMarker, state: ButtonHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
