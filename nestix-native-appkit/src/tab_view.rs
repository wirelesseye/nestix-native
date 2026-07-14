use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{
    Element, Layout, Readonly, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{
    Dimension, StyleContext, StyleScope, matched_style, resolved_view_style, style_align_self,
    style_dimension, style_flex_basis, style_flex_grow, style_flex_shrink, style_margin,
};
use nestix_native_core::{TabViewItemProps, TabViewProps, TreeContext};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSTabView, NSTabViewDelegate, NSTabViewItem, NSView};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use taffy::{NodeId, Size, Style, prelude::FromLength};

use crate::{WindowContext, contexts::ParentContext};
use nestix_native_core::utils::{inset_to_taffy, margin_to_taffy};

thread_local! {
    static DELEGATES: RefCell<HashMap<String, Retained<TabViewDelegate>>> = RefCell::new(HashMap::new());
}

struct TabViewContext {
    current_selected: Readonly<Option<String>>,
    subtrees: Rc<RefCell<HashMap<String, Rc<TreeContext>>>>,
}

#[component]
pub fn TabView(props: &TabViewProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__TabView", "__appkit_TabView"];

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
    let effective_style = resolved_view_style(style_props.clone(), &props.view);

    let current_selected = create_state(None);
    let subtrees = Rc::new(RefCell::new(HashMap::new()));

    let mtm = MainThreadMarker::new().unwrap();
    let view = NNTabView::new(
        mtm,
        NNTabViewState {
            subtrees: subtrees.clone(),
        },
    );
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
    element.on_place(closure!(
        [view, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&view, Some(node_id), index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&view, Some(node_id));
            }
        }
    ));

    element.on_unmount(closure!(
        [view, parent_context] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&view, Some(node_id));
            }
            DELEGATES.with_borrow_mut(|delegates| delegates.remove(&tab_view_id));
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
            tree_context,
            parent_context.parent_node,
            style_props,
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
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

            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: width.to_taffy(scale_factor),
                        height: height.to_taffy(scale_factor),
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
        [tree_context, parent_context.parent_node, view] || {
            if parent_node.is_some()
                && let Some(layout) = tree_context.layout(node_id)
            {
                view.setFrame(NSRect::new(
                    NSPoint::new(layout.location.x.into(), layout.location.y.into()),
                    NSSize::new(layout.size.width.into(), layout.size.height.into()),
                ));
            }
        }
    );

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style
        ) {
            ContextProvider<TabViewContext>(
                TabViewContext {
                    current_selected: current_selected.into_readonly(),
                    subtrees,
                }
            ) {
                ContextProvider<ParentContext>(ParentContext {
                    add_child: Some(callback!([view] |child: &NSObject, _: Option<NodeId>| {
                        let item = child.downcast_ref::<NSTabViewItem>().unwrap();
                        if view.tabViewItems().containsObject(item) {
                            view.removeTabViewItem(item);
                        }
                        view.addTabViewItem(item);
                    })),
                    insert_child: Some(callback!([view] |child: &NSObject, _: Option<NodeId>, index: usize| {
                        let item = child.downcast_ref::<NSTabViewItem>().unwrap();
                        if view.tabViewItems().containsObject(item) {
                            view.removeTabViewItem(item);
                        }
                        view.insertTabViewItem_atIndex(item, index as _);
                    })),
                    remove_child: Some(callback!([view] |child: &NSObject, _: Option<NodeId>| {
                        let item = child.downcast_ref::<NSTabViewItem>().unwrap();
                        view.removeTabViewItem(item);
                    })),
                    parent_node: Some(node_id),
                }) {
                    $(props.children.clone())
                }
            }
        }
    }
}

struct NNTabViewState {
    subtrees: Rc<RefCell<HashMap<String, Rc<TreeContext>>>>,
}

define_class!(
    #[unsafe(super = NSTabView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NNTabViewState]
    struct NNTabView;

    unsafe impl NSObjectProtocol for NNTabView {}

    impl NNTabView {
        #[unsafe(method(layout))]
        fn layout(&self) {
            unsafe {
                let _: () = msg_send![super(self), layout];
            }

            let subtrees = self.ivars().subtrees.borrow();
            for item in self.tabViewItems().iter() {
                let Some(identifier) = item.identifier() else {
                    continue;
                };
                let Some(identifier) = identifier.downcast_ref::<NSString>() else {
                    continue;
                };
                let Some(subtree_context) = subtrees.get(&identifier.to_string()) else {
                    continue;
                };
                let Some(root_node) = subtree_context.root_node() else {
                    continue;
                };
                let Some(item_view) = item.view(self.mtm()) else {
                    continue;
                };

                let size = item_view.frame().size;
                subtree_context.update_style(root_node, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(size.width as f32),
                        height: taffy::Dimension::from_length(size.height as f32),
                    },
                    ..prev
                });
                subtree_context.refresh();
            }
        }
    }
);

impl NNTabView {
    fn new(mtm: MainThreadMarker, state: NNTabViewState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

impl AsRef<NSObject> for NNTabView {
    fn as_ref(&self) -> &NSObject {
        self
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
    const DEFAULT_CLASSES: [&str; 2] = ["__TabViewItem", "__appkit_TabViewItem"];

    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let tab_view_context = element.context::<TabViewContext>().unwrap();
    let mtm = MainThreadMarker::new().unwrap();

    let id = NSString::from_str(&props.id.get());
    let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), Some(&id)) };
    element.provide_handle(item.as_ref() as *const NSObject);

    element.on_place(closure!(
        [item, parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert_child) = &parent_context.insert_child
            {
                insert_child(&item, None, index);
            } else if let Some(add_child) = &parent_context.add_child {
                add_child(&item, None);
            }
        }
    ));

    let subtree_context = Rc::new(TreeContext::new());
    tab_view_context
        .subtrees
        .borrow_mut()
        .insert(props.id.get(), subtree_context.clone());

    element.on_unmount(closure!(
        [parent_context, item, tab_view_context, props.id] || {
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(&item, None);
            }
            tab_view_context.subtrees.borrow_mut().remove(&id.get());
        }
    ));

    scoped_effect!(
        element,
        [item, props.id] || {
            let ns_id = NSString::from_str(&id.get());
            unsafe {
                item.setIdentifier(Some(&ns_id));
            }
        }
    );

    scoped_effect!(
        element,
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
                            width: taffy::Dimension::from_length(frame.size.width as f32),
                            height: taffy::Dimension::from_length(frame.size.height as f32),
                        },
                        ..prev
                    });
                    subtree_context.refresh();
                }
            }
        }
    );

    scoped_effect!(
        element,
        [item, props.title] || {
            let ns_string = NSString::from_str(&title.get());
            item.setLabel(&ns_string);
        }
    );

    scoped_effect!(
        element,
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
                                width: taffy::Dimension::from_length(frame.size.width as f32),
                                height: taffy::Dimension::from_length(frame.size.height as f32),
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
        StyleScope(.class = props.class.clone(), .default_classes = DEFAULT_CLASSES) {
            ContextProvider<TreeContext>(subtree_context.clone()) {
                ContextProvider<ParentContext>(ParentContext {
                    add_child: Some(callback!([item, subtree_context] |object: &NSObject, child_node: Option<NodeId>| {
                        let view = object.downcast_ref::<NSView>().unwrap();
                        item.setView(Some(view));
                        subtree_context.set_root_node(child_node);

                        let frame = view.frame();
                        if let Some(child_node) = child_node {
                            subtree_context.update_style(child_node, |prev| Style {
                                size: Size {
                                    width: taffy::Dimension::from_length(frame.size.width as f32),
                                    height: taffy::Dimension::from_length(frame.size.height as f32)
                                },
                                ..prev
                            });
                            subtree_context.refresh();
                        }

                        if let Some(tab_view) = item.tabView(mtm) {
                            tab_view.setNeedsLayout(true);
                            tab_view.layoutSubtreeIfNeeded();
                        }
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([item] |_: &NSObject, _: Option<NodeId>| {
                        item.setView(None);
                        subtree_context.set_root_node(None);
                    })),
                    parent_node: None,
                }) {
                    $(props.children.clone().map(|element| Layout::from(element.clone())))
                }
            }
        }
    }
}
