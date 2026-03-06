use nestix::{Element, closure, component, effect};
use nestix_native_core::{Dimension, ExtendsViewProps, LabelProps};
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{
    WindowContext,
    contexts::{ParentContext, TreeContext},
};

#[component]
pub fn Label(props: &LabelProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();
    let ns_string = NSString::from_str(&props.text.get());
    let label = NSTextField::labelWithString(&ns_string, mtm);
    element.provide_handle(label.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(true);
    if let Some(add_child) = &parent_context.add_child {
        add_child(&label, Some(node_id));
    }

    element.on_destroy(closure!(
        [parent_context, label] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&label, Some(node_id));
            }
        }
    ));

    // effect!(
    //     [window_context.scale_factor, props.x(), props.y()] || {
    //         let scale_factor = scale_factor.get();
    //         let x: f64 = match x.get() {
    //             Dimension::Auto => 0.0,
    //             Dimension::Length(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
    //         };
    //         let y: f64 = match y.get() {
    //             Dimension::Auto => 0.0,
    //             Dimension::Length(pixel_unit) => pixel_unit.to_logical(scale_factor).0,
    //         };
    //     }
    // );

    effect!(
        [
            window_context.scale_factor,
            tree_context,
            label,
            props.width(),
            props.height()
        ] || {
            let scale_factor = scale_factor.get();

            if width.get().is_auto() || height.get().is_auto() {
                label.sizeToFit();
            }
            let width = match width.get() {
                Dimension::Auto => label.frame().size.width as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height.get() {
                Dimension::Auto => label.frame().size.height as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };

            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width: taffy::Dimension::from_length(width),
                    height: taffy::Dimension::from_length(height),
                },
                ..prev
            });

            tree_context.update();
        }
    );

    effect!(
        [tree_context, label] || {
            if let Some(layout) = tree_context.layout(node_id) {
                label.setFrame(NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                ));
            }
        }
    );

    effect!(
        [window_context.scale_factor, label, props.text, props.width(), props.height()] || {
            let ns_string = NSString::from_str(&text.get());
            label.setStringValue(&ns_string);

            if width.get().is_auto() || height.get().is_auto() {
                let scale_factor = scale_factor.get();
                label.sizeToFit();

                let width = match width.get() {
                    Dimension::Auto => label.frame().size.width as f32,
                    Dimension::Length(pixel_unit) => {
                        pixel_unit.to_logical::<f32>(scale_factor).into()
                    }
                };
                let height = match height.get() {
                    Dimension::Auto => label.frame().size.height as f32,
                    Dimension::Length(pixel_unit) => {
                        pixel_unit.to_logical::<f32>(scale_factor).into()
                    }
                };

                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    ..prev
                });

                tree_context.update();
            }
        }
    );
}
