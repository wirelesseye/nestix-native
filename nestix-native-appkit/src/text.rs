use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::{Dimension, TextProps, TreeContext};
use objc2::MainThreadMarker;
use objc2_app_kit::NSTextField;
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, utils::margin_to_taffy};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let mtm = MainThreadMarker::new().unwrap();
    let ns_string = NSString::from_str(&props.text.get());
    let label = NSTextField::labelWithString(&ns_string, mtm);
    element.provide_handle(label.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [label, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&label, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&label, Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [parent_context, label] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&label, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            parent_context.parent_node,
            tree_context,
            label,
            props.view.width,
            props.view.height,
            props.text,
        ] || {
            let scale_factor = scale_factor.get();
            let ns_string = NSString::from_str(&text.get());
            label.setStringValue(&ns_string);

            let intrinsic_size = (width.get().is_auto() || height.get().is_auto())
                .then(|| label.intrinsicContentSize());
            let width = match width.get() {
                Dimension::Auto => intrinsic_size.unwrap().width as f32,
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height.get() {
                Dimension::Auto => intrinsic_size.unwrap().height as f32,
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
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(margin.get(), scale_factor),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, props.view.align_self] || {
            tree_context.update_style(node_id, |prev| Style {
                align_self: align_self.get().to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [tree_context, parent_context.parent_node, label] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let alignment_rect = NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                );
                label.setFrame(label.frameForAlignmentRect(alignment_rect));
            }
        }
    );
}
