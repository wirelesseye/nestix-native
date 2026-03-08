use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{
    Element, Readonly, State, callback, closure, component, components::ContextProvider,
    create_state, effect, layout,
};
use nestix_native_core::{TabViewItemProps, TabViewProps, TreeContext};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSTabView, NSTabViewDelegate, NSTabViewItem, NSView};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use taffy::{Dimension, NodeId, Size, Style, prelude::FromLength};

use crate::contexts::ParentContext;

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<TabViewDelegate>>> = RefCell::new(HashMap::new());
}

struct TabViewContext {
    current_selected: Readonly<Option<String>>,
}

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();

    let current_selected = create_state(None);

    let mtm = MainThreadMarker::new().unwrap();
    let view = NSTabView::new(mtm);
    let delegate = TabViewDelegate::new(
        mtm,
        TabViewState {
            current_selected: current_selected.clone(),
        },
    );
    view.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    let tab_view_id = nanoid::nanoid!();
    DELEGATES.with_borrow_mut(
        |handlers: &mut HashMap<String, Retained<TabViewDelegate>>| {
            handlers.insert(tab_view_id.clone(), delegate)
        },
    );

    element.provide_handle(view.as_ref() as *const NSObject);

    let node_id = tree_context.create_node(true);
    if let Some(add_child) = &parent_context.add_child {
        add_child(&view, Some(node_id));
    }

    element.on_destroy(closure!(
        [view] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&view, Some(node_id));
            }
            DELEGATES.with_borrow_mut(|delegates| delegates.remove(&tab_view_id));
        }
    ));

    layout! {
        ContextProvider<TabViewContext>(
            .value = TabViewContext {
                current_selected: current_selected.into_readonly()
            }
        ) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    add_child: Some(callback!([view] |child: &NSObject, _: Option<NodeId>| {
                        view.addTabViewItem(child.downcast_ref::<NSTabViewItem>().unwrap());
                    })),
                    remove_child: Some(callback!([view] |child: &NSObject, _: Option<NodeId>| {
                        view.removeTabViewItem(child.downcast_ref::<NSTabViewItem>().unwrap());
                    })),
                    parent_node: Some(node_id),
                },
                .children = props.children.clone(),
            )
        }
    }
}

struct TabViewState {
    current_selected: State<Option<String>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "TabViewDelegate"]
    #[ivars = TabViewState]
    struct TabViewDelegate;

    unsafe impl NSObjectProtocol for TabViewDelegate {}

    unsafe impl NSTabViewDelegate for TabViewDelegate {
        #[unsafe(method(tabView:didSelectTabViewItem:))]
        fn tab_view_did_select_tab_view_item(&self, _: &NSTabView, tab_view_item: &NSTabViewItem) {
            let id = tab_view_item.identifier();
            if let Some(id) = id {
                let ns_string = id.downcast_ref::<NSString>().unwrap();
                self.ivars()
                    .current_selected
                    .set(Some(ns_string.to_string()));
            }
        }
    }
);

impl TabViewDelegate {
    fn new(mtm: MainThreadMarker, state: TabViewState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

#[component]
pub fn TabViewItem(props: &TabViewItemProps, element: &Element) -> Element {
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_view_context = element.context::<TabViewContext>().unwrap();
    let mtm = MainThreadMarker::new().unwrap();

    let id = NSString::from_str(&props.id.get());
    let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), Some(&id)) };
    element.provide_handle(item.as_ref() as *const NSObject);

    if let Some(add_child) = &parent_context.add_child {
        add_child(&item, None);
    }

    let subtree_context = Rc::new(TreeContext::new());

    element.on_destroy(closure!(
        [parent_context, item] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&item, None);
            }
        }
    ));

    effect!(
        [
            props.id,
            tab_view_context.current_selected,
            subtree_context,
            item
        ] || {
            if current_selected.get() == Some(id.get()) {
                if let Some(root_node) = subtree_context.root_node() {
                    let frame = item.view(mtm).unwrap().frame();
                    subtree_context.update_style(root_node, |prev| Style {
                        size: Size {
                            width: Dimension::from_length(frame.size.width as f32),
                            height: Dimension::from_length(frame.size.height as f32),
                        },
                        ..prev
                    });
                    subtree_context.refresh();
                }
            }
        }
    );

    effect!(
        [item, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            item.setLabel(&ns_string);
        }
    );

    effect!(
        [
            tree_context,
            subtree_context,
            parent_context.parent_node,
            item
        ] || {
            if let Some(parent_node) = parent_node {
                if tree_context.layout(parent_node).is_some() {
                    if let Some(root_node) = subtree_context.root_node() {
                        let frame = item.view(mtm).unwrap().frame();
                        subtree_context.update_style(root_node, |prev| Style {
                            size: Size {
                                width: Dimension::from_length(frame.size.width as f32),
                                height: Dimension::from_length(frame.size.height as f32),
                            },
                            ..prev
                        });
                        subtree_context.refresh();
                    }
                }
            }
        }
    );

    layout! {
        ContextProvider<TreeContext>(.value = subtree_context.clone()) {
            ContextProvider<ParentContext>(
                .value = ParentContext {
                    add_child: Some(callback!([item] |object: &NSObject, child_node: Option<NodeId>| {
                        let view = object.downcast_ref::<NSView>().unwrap();
                        item.setView(Some(view));
                        subtree_context.set_root_node(child_node);

                        let frame = view.frame();
                        if let Some(child_node) = child_node {
                            subtree_context.update_style(child_node, |prev| Style {
                                size: Size {
                                    width: Dimension::from_length(frame.size.width as f32),
                                    height: Dimension::from_length(frame.size.height as f32)
                                },
                                ..prev
                            });
                        }
                    })),
                    remove_child: None,
                    parent_node: None,
                },
            ) {
                $(props.children.get())
            }
        }
    }
}
