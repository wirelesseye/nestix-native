use gtk4::{gdk_pixbuf::prelude::PixbufLoaderExt, prelude::*};
use nestix::{Element, closure, component, create_state, scoped_effect};
use nestix_native_core::{
    ContentFit, ImageSource, ImageViewProps, StyleContext, TreeContext, WithAuto, matched_style,
    style_align_self, style_flex_basis, style_flex_grow, style_flex_shrink, style_length_with_auto,
    style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{
    Size, Style,
    prelude::{FromLength, FromPercent, TaffyAuto},
};

use crate::{
    WindowContext,
    allocation_bin::AllocationBin,
    contexts::{LayoutRefreshContext, ParentContext},
};

#[component]
pub fn ImageView(props: &ImageViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__ImageView", "__gtk4_ImageView"];

    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let layout_refresh = element.context::<LayoutRefreshContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_props = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let host = AllocationBin::new();
    host.set_overflow(gtk4::Overflow::Hidden);
    let picture_host = gtk4::CenterBox::new();
    let picture = gtk4::Picture::new();
    picture.set_can_shrink(true);
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture_host.set_center_widget(Some(&picture));
    host.set_child(Some(picture_host.upcast_ref()));
    let widget: gtk4::Widget = host.clone().upcast();
    let node_id = tree_context.create_node(true);
    let natural_size = create_state((0, 0));
    element.provide_handle(widget.clone());

    element.on_place(closure!(
        [widget, parent_context] | placement | {
            parent_context.place_child(&widget, Some(node_id), placement);
        }
    ));
    element.on_unmount(closure!(
        [widget, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&widget, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        [picture, natural_size, props.source] || {
            if let Some((texture, width, height)) = load_image(source.get()) {
                picture.set_paintable(Some(&texture));
                natural_size.set((width, height));
            } else {
                picture.set_paintable(gtk4::gdk::Paintable::NONE);
                natural_size.set((0, 0));
            }
        }
    );

    scoped_effect!(
        [picture, props.content_fit] || {
            let content_fit = content_fit.get();
            let intrinsic_size = content_fit == ContentFit::None;
            picture.set_content_fit(gtk_content_fit(content_fit));
            picture.set_can_shrink(!intrinsic_size);
            picture.set_hexpand(!intrinsic_size);
            picture.set_vexpand(!intrinsic_size);
            picture.set_halign(if intrinsic_size {
                gtk4::Align::Center
            } else {
                gtk4::Align::Fill
            });
            picture.set_valign(if intrinsic_size {
                gtk4::Align::Center
            } else {
                gtk4::Align::Fill
            });
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
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
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            parent_context.parent_node,
            style_props,
            natural_size,
            props.view.width,
            props.view.height,
            window_context.scale_factor
        ] || {
            let (natural_width, natural_height) = natural_size.get();
            let style_props = style_props.get();
            let width = style_length_with_auto(
                style_props.as_ref(),
                width.get(),
                WithAuto::Auto,
                |style| style.width,
            );
            let height = style_length_with_auto(
                style_props.as_ref(),
                height.get(),
                WithAuto::Auto,
                |style| style.height,
            );
            let width_is_auto = width.is_auto();
            let height_is_auto = height.is_auto();
            let ratio = if natural_height > 0 {
                natural_width as f32 / natural_height as f32
            } else {
                1.0
            };
            let (width, height) = match (width, height) {
                (WithAuto::Auto, WithAuto::Auto) => (natural_width as f32, natural_height as f32),
                (WithAuto::Value(width), WithAuto::Auto) => {
                    let width = width.to_logical::<f32>(scale_factor.get()).0;
                    (width, width / ratio)
                }
                (WithAuto::Auto, WithAuto::Value(height)) => {
                    let height = height.to_logical::<f32>(scale_factor.get()).0;
                    (height * ratio, height)
                }
                (WithAuto::Value(width), WithAuto::Value(height)) => (
                    width.to_logical::<f32>(scale_factor.get()).0,
                    height.to_logical::<f32>(scale_factor.get()).0,
                ),
            };
            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    max_size: Size {
                        width: if width_is_auto {
                            taffy::Dimension::from_percent(1.0_f32)
                        } else {
                            taffy::Dimension::AUTO
                        },
                        height: if height_is_auto {
                            taffy::Dimension::from_percent(1.0_f32)
                        } else {
                            taffy::Dimension::AUTO
                        },
                    },
                    item_is_replaced: true,
                    aspect_ratio: Some(ratio),
                    ..prev
                });
            }
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.view.left,
            props.view.top,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            let left =
                style_length_with_auto(style_props.as_ref(), left.get(), WithAuto::Auto, |style| {
                    style.left
                });
            let top =
                style_length_with_auto(style_props.as_ref(), top.get(), WithAuto::Auto, |style| {
                    style.top
                });
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor.get()),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.view.margin(),
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            layout_refresh,
            style_props,
            props.view.align_self
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            layout_refresh.queue_refresh();
        }
    );

    scoped_effect!(
        [
            tree_context,
            parent_context.parent_node,
            parent_context.fixed,
            host,
        ] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                let width = layout.size.width.round() as i32;
                let height = layout.size.height.round() as i32;
                host.set_size_request(width, height);
                if let Some(fixed) = &fixed
                    && host.parent().as_ref() == Some(fixed.upcast_ref())
                {
                    fixed.move_(&host, layout.location.x as f64, layout.location.y as f64);
                    fixed.queue_allocate();
                }
                host.queue_allocate();
            }
        }
    );
}

fn load_image(source: ImageSource) -> Option<(gtk4::gdk::Texture, i32, i32)> {
    let pixbuf = match source {
        ImageSource::File(path) => gtk4::gdk_pixbuf::Pixbuf::from_file(path).ok()?,
        ImageSource::Bytes(bytes) => {
            let loader = gtk4::gdk_pixbuf::PixbufLoader::new();
            loader.write(&bytes).ok()?;
            loader.close().ok()?;
            loader.pixbuf()?
        }
    };
    let width = pixbuf.width();
    let height = pixbuf.height();
    Some((gtk4::gdk::Texture::for_pixbuf(&pixbuf), width, height))
}

fn gtk_content_fit(content_fit: ContentFit) -> gtk4::ContentFit {
    match content_fit {
        ContentFit::Contain => gtk4::ContentFit::Contain,
        ContentFit::Cover => gtk4::ContentFit::Cover,
        ContentFit::Fill | ContentFit::None => gtk4::ContentFit::Fill,
        ContentFit::ScaleDown => gtk4::ContentFit::ScaleDown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_fit_maps_to_gtk() {
        assert_eq!(
            gtk_content_fit(ContentFit::Contain),
            gtk4::ContentFit::Contain
        );
        assert_eq!(gtk_content_fit(ContentFit::Cover), gtk4::ContentFit::Cover);
        assert_eq!(gtk_content_fit(ContentFit::Fill), gtk4::ContentFit::Fill);
        assert_eq!(
            gtk_content_fit(ContentFit::ScaleDown),
            gtk4::ContentFit::ScaleDown
        );
        assert_eq!(gtk_content_fit(ContentFit::None), gtk4::ContentFit::Fill);
    }
}
