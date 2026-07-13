use std::{cell::RefCell, rc::Rc};

use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::{
    ContentFit, Dimension, ImageSource, ImageViewProps, StyleContext, TreeContext, matched_style,
    style_align_self, style_dimension, style_flex_basis, style_flex_grow, style_flex_shrink,
    style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSImage, NSImageScaling, NSImageView};
use objc2_foundation::{NSData, NSObject, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext};

#[component]
pub fn ImageView(props: &ImageViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__ImageView", "__appkit_ImageView"];

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
    let empty = NSImage::initWithSize(NSImage::alloc(), NSSize::new(0.0, 0.0));
    let image_view = NSImageView::imageViewWithImage(&empty, mtm);
    image_view.setClipsToBounds(true);
    element.provide_handle(image_view.as_ref() as *const NSObject);

    // NSImage::size can be changed to implement aspect-fill, so retain the
    // decoded image's original size separately.
    let natural_size = Rc::new(RefCell::new(NSSize::new(0.0, 0.0)));

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [image_view, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&image_view, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&image_view, Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [parent_context, image_view] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&image_view, Some(node_id));
            }
        }
    ));

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
            parent_context.parent_node,
            tree_context,
            style_props,
            image_view,
            natural_size,
            props.source,
            props.content_fit,
            props.view.width,
            props.view.height,
        ] || {
            let loaded = match source.get() {
                ImageSource::File(path) => {
                    let path = NSString::from_str(&path.to_string_lossy());
                    NSImage::initWithContentsOfFile(NSImage::alloc(), &path)
                }
                ImageSource::Bytes(bytes) => {
                    let data =
                        unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
                    NSImage::initWithData(NSImage::alloc(), &data)
                }
            };

            let Some(image) = loaded else {
                image_view.setImage(None);
                *natural_size.borrow_mut() = NSSize::new(0.0, 0.0);
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(0.0),
                        height: taffy::Dimension::from_length(0.0),
                    },
                    ..prev
                });
                tree_context.refresh();
                return;
            };

            let intrinsic = image.size();
            *natural_size.borrow_mut() = intrinsic;
            image_view.setImage(Some(&image));
            image_view.setImageScaling(match content_fit.get() {
                ContentFit::Contain | ContentFit::Cover => {
                    NSImageScaling::ScaleProportionallyUpOrDown
                }
                ContentFit::Fill => NSImageScaling::ScaleAxesIndependently,
                ContentFit::None => NSImageScaling::ScaleNone,
                ContentFit::ScaleDown => NSImageScaling::ScaleProportionallyDown,
            });

            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let width = style_dimension(style_props.as_ref(), width.get(), Dimension::Auto, |s| {
                s.width
            });
            let height =
                style_dimension(style_props.as_ref(), height.get(), Dimension::Auto, |s| {
                    s.height
                });
            let ratio = if intrinsic.height > 0.0 {
                intrinsic.width / intrinsic.height
            } else {
                1.0
            };
            let (width, height) = match (width, height) {
                (Dimension::Auto, Dimension::Auto) => {
                    (intrinsic.width as f32, intrinsic.height as f32)
                }
                (Dimension::Length(width), Dimension::Auto) => {
                    let width = width.to_logical::<f32>(scale_factor).0;
                    (width, width / ratio as f32)
                }
                (Dimension::Auto, Dimension::Length(height)) => {
                    let height = height.to_logical::<f32>(scale_factor).0;
                    (height * ratio as f32, height)
                }
                (Dimension::Length(width), Dimension::Length(height)) => (
                    width.to_logical::<f32>(scale_factor).0,
                    height.to_logical::<f32>(scale_factor).0,
                ),
            };

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    item_is_replaced: true,
                    aspect_ratio: Some(ratio as f32),
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
            let left = style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |s| {
                s.left
            });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |s| s.top);
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
        [
            tree_context,
            image_view,
            parent_context.parent_node,
            natural_size,
            props.content_fit
        ] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let frame_size = NSSize::new(layout.size.width.into(), layout.size.height.into());
                if let Some(image) = image_view.image() {
                    let natural = *natural_size.borrow();
                    if content_fit.get() == ContentFit::Cover
                        && natural.width > 0.0
                        && natural.height > 0.0
                    {
                        let scale = (frame_size.width / natural.width)
                            .max(frame_size.height / natural.height);
                        image.setSize(NSSize::new(natural.width * scale, natural.height * scale));
                        image_view.setImageScaling(NSImageScaling::ScaleNone);
                    } else {
                        image.setSize(natural);
                    }
                }
                image_view.setFrame(NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    frame_size,
                ));
            }
        }
    );
}

#[cfg(test)]
mod tests {
    use taffy::{FlexDirection, TaffyTree, prelude::*};

    #[test]
    fn auto_sized_large_image_does_not_collapse_sibling_controls() {
        let mut tree = TaffyTree::<()>::new();
        let text = tree
            .new_leaf(Style {
                size: Size {
                    width: length(80.0),
                    height: length(20.0),
                },
                ..Style::default()
            })
            .unwrap();
        let button = tree
            .new_leaf(Style {
                size: Size {
                    width: length(100.0),
                    height: length(32.0),
                },
                ..Style::default()
            })
            .unwrap();
        let image = tree
            .new_leaf(Style {
                item_is_replaced: true,
                size: Size {
                    width: length(3840.0),
                    height: length(2160.0),
                },
                max_size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                flex_basis: percent(1.0),
                flex_shrink: 1000.0,
                aspect_ratio: Some(3840.0 / 2160.0),
                ..Style::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: Size {
                        width: length(500.0),
                        height: length(350.0),
                    },
                    ..Style::default()
                },
                &[text, button, image],
            )
            .unwrap();

        tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

        let text_layout = tree.layout(text).unwrap();
        let button_layout = tree.layout(button).unwrap();
        let image_layout = tree.layout(image).unwrap();
        assert_eq!(text_layout.size.height, 20.0);
        assert_eq!(button_layout.size.height, 32.0);
        assert!(image_layout.size.width <= 500.0);
        assert!(image_layout.size.height <= 350.0);
    }
}
