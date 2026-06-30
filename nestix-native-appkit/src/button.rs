use std::{cell::RefCell, collections::HashMap};

use nestix::{Element, PropValue, Shared, closure, component, scoped_effect};
use nestix_native_core::{ButtonProps, Dimension, TreeContext, ViewPropsExt};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel,
};
use objc2_app_kit::NSButton;
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use taffy::{Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext, utils::margin_to_taffy};

thread_local! {
    static HANDLERS: RefCell<HashMap<String, Retained<ButtonHandler>>> = RefCell::new(HashMap::new());
}

const NATIVE_BUTTON_MARGIN_LEFT_OFFSET: f32 = 6.0;
const NATIVE_BUTTON_MARGIN_RIGHT_OFFSET: f32 = 6.0;
const NATIVE_BUTTON_MARGIN_TOP_OFFSET: f32 = 4.0;
const NATIVE_BUTTON_MARGIN_BOTTOM_OFFSET: f32 = 6.0;

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

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
    element.provide_handle(button.as_ref() as *const NSObject);

    let button_id = nanoid::nanoid!();
    HANDLERS.with_borrow_mut(|handlers| handlers.insert(button_id.clone(), handler));

    let node_id = tree_context.create_node(true);
    element.on_place(closure!(
        [button, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&button, Some(node_id), index);
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
        [
            window_context.scale_factor,
            tree_context,
            parent_context.parent_node,
            button,
            props.width(),
            props.height(),
        ] || {
            let scale_factor = scale_factor.get();

            if width.get().is_auto() || height.get().is_auto() {
                button.sizeToFit();
            }
            let width = match width.get() {
                Dimension::Auto => {
                    let native_width = button.frame().size.width as f32;
                    (native_width
                        - NATIVE_BUTTON_MARGIN_LEFT_OFFSET
                        - NATIVE_BUTTON_MARGIN_RIGHT_OFFSET)
                        .max(0.0)
                }
                Dimension::Length(pixel_unit) => pixel_unit.to_logical::<f32>(scale_factor).into(),
            };
            let height = match height.get() {
                Dimension::Auto => {
                    let native_height = button.frame().size.height as f32;
                    (native_height
                        - NATIVE_BUTTON_MARGIN_TOP_OFFSET
                        - NATIVE_BUTTON_MARGIN_BOTTOM_OFFSET)
                        .max(0.0)
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
            props.view_props().margin()
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
        [tree_context, props.align_self()] || {
            tree_context.update_style(node_id, |prev| Style {
                align_self: align_self.get().to_taffy(),
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
                button.setFrame(NSRect::new(
                    NSPoint::new(
                        (layout.location.x - NATIVE_BUTTON_MARGIN_LEFT_OFFSET).into(),
                        (layout.location.y - NATIVE_BUTTON_MARGIN_TOP_OFFSET).into(),
                    ),
                    NSSize::new(
                        (layout.size.width
                            + NATIVE_BUTTON_MARGIN_LEFT_OFFSET
                            + NATIVE_BUTTON_MARGIN_RIGHT_OFFSET)
                            .into(),
                        (layout.size.height
                            + NATIVE_BUTTON_MARGIN_TOP_OFFSET
                            + NATIVE_BUTTON_MARGIN_BOTTOM_OFFSET)
                            .into(),
                    ),
                ));
            }
        }
    );

    scoped_effect!(
        element,
        [button, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            button.setTitle(&ns_string);
        }
    );
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

impl ButtonHandler {
    fn new(mtm: MainThreadMarker, state: ButtonHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}
