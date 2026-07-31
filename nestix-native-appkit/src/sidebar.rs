use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use nestix::{
    Element, Layout, PropValue, Shared, callback, closure, component, components::ContextProvider,
    layout, scoped_effect,
};
use nestix_native_core::{SidebarProps, TreeContext};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{
    NSSplitView, NSSplitViewController, NSSplitViewItem, NSView, NSViewController,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol};
use taffy::NodeId;

use crate::{WindowContext, contexts::ParentContext, window::ContentHost};

pub(crate) struct MountedSidebar {
    owner: String,
    controller: Retained<SidebarSplitViewController>,
}

struct SidebarSplitViewControllerState {
    sidebar_item: RefCell<Option<Retained<NSSplitViewItem>>>,
    open: PropValue<Option<bool>>,
    on_open_change: PropValue<Option<Shared<dyn Fn(bool)>>>,
    width: PropValue<Option<f64>>,
    min_width: PropValue<Option<f64>>,
    resizable: PropValue<bool>,
    native_min_width: Cell<Option<f64>>,
    last_open: Cell<Option<bool>>,
}

define_class!(
    #[unsafe(super = NSSplitViewController)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SidebarSplitViewControllerState]
    struct SidebarSplitViewController;

    unsafe impl NSObjectProtocol for SidebarSplitViewController {}

    impl SidebarSplitViewController {
        #[unsafe(method(splitViewDidResizeSubviews:))]
        fn split_view_did_resize_subviews(&self, notification: &NSNotification) {
            unsafe {
                let _: () = msg_send![super(self), splitViewDidResizeSubviews: notification];
                let _self = Retained::retain(std::ptr::from_ref(self).cast_mut())
                    .expect("sidebar controller must remain alive during resize handling");
                self.report_native_open_change();
            }
        }

        #[unsafe(method(splitView:constrainSplitPosition:ofSubviewAt:))]
        fn split_view_constrain_position(
            &self,
            split_view: &NSSplitView,
            proposed_position: f64,
            divider_index: isize,
        ) -> f64 {
            let _ = split_view;
            if divider_index != 0 || self.ivars().resizable.get() {
                return proposed_position;
            }

            let Some(item) = self.ivars().sidebar_item.borrow().as_ref().cloned() else {
                return proposed_position;
            };
            if item.isCollapsed() {
                return proposed_position;
            }
            if let Some(width) = self.effective_width() {
                return width;
            }

            let current_width = item.viewController(self.mtm()).view().frame().size.width;
            if current_width > 0.0 {
                current_width.max(self.effective_min_width().unwrap_or(0.0))
            } else {
                proposed_position
            }
        }
    }
);

impl SidebarSplitViewController {
    fn new(
        mtm: MainThreadMarker,
        open: PropValue<Option<bool>>,
        on_open_change: PropValue<Option<Shared<dyn Fn(bool)>>>,
        width: PropValue<Option<f64>>,
        min_width: PropValue<Option<f64>>,
        resizable: PropValue<bool>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SidebarSplitViewControllerState {
            sidebar_item: RefCell::new(None),
            open,
            on_open_change,
            width,
            min_width,
            resizable,
            native_min_width: Cell::new(None),
            last_open: Cell::new(None),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn set_sidebar_item(&self, item: Retained<NSSplitViewItem>) {
        self.ivars()
            .native_min_width
            .set(Some(item.minimumThickness()));
        if let Some(open) = self.ivars().open.get() {
            item.setCollapsed(!open);
        }
        self.ivars().last_open.set(Some(!item.isCollapsed()));
        self.ivars().sidebar_item.replace(Some(item));
    }

    fn effective_min_width(&self) -> Option<f64> {
        validated_width("Sidebar min_width", self.ivars().min_width.get())
    }

    fn effective_width(&self) -> Option<f64> {
        clamped_width(self.ivars().width.get(), self.ivars().min_width.get())
    }

    fn apply_sizing_props(&self) {
        let Some(item) = self.ivars().sidebar_item.borrow().as_ref().cloned() else {
            return;
        };
        let minimum = self
            .effective_min_width()
            .or(self.ivars().native_min_width.get());
        if let Some(minimum) = minimum {
            item.setMinimumThickness(minimum);
        }

        if item.isCollapsed() {
            return;
        }
        let requested = self.effective_width().or_else(|| {
            let current = item.viewController(self.mtm()).view().frame().size.width;
            self.effective_min_width()
                .filter(|minimum| current < *minimum)
        });
        if let Some(width) = requested {
            self.splitView().setPosition_ofDividerAtIndex(width, 0);
        }
    }

    fn apply_open_prop(&self, open: Option<bool>) {
        let Some(open) = open else {
            return;
        };
        let Some(item) = self.ivars().sidebar_item.borrow().as_ref().cloned() else {
            return;
        };
        if item.isCollapsed() == open {
            self.ivars().last_open.set(Some(open));
            item.setCollapsed(!open);
            if open {
                self.apply_sizing_props();
            }
        }
    }

    fn report_native_open_change(&self) {
        let Some(item) = self.ivars().sidebar_item.borrow().as_ref().cloned() else {
            return;
        };
        let open = !item.isCollapsed();
        if self.ivars().last_open.replace(Some(open)) == Some(open) {
            return;
        }

        let requested = self.ivars().open.get();
        if requested != Some(open)
            && let Some(on_open_change) = self.ivars().on_open_change.get()
        {
            on_open_change(open);
        }

        if let Some(requested) = self.ivars().open.get()
            && requested != open
        {
            self.apply_open_prop(Some(requested));
        }
    }
}

fn validated_width(name: &str, value: Option<f64>) -> Option<f64> {
    value.map(|value| {
        assert!(
            value.is_finite() && value >= 0.0,
            "{name} must be a finite, non-negative number"
        );
        value
    })
}

fn clamped_width(width: Option<f64>, min_width: Option<f64>) -> Option<f64> {
    validated_width("Sidebar width", width)
        .map(|width| width.max(validated_width("Sidebar min_width", min_width).unwrap_or(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_width_is_clamped_to_the_minimum() {
        assert_eq!(clamped_width(Some(180.0), Some(220.0)), Some(220.0));
        assert_eq!(clamped_width(Some(280.0), Some(220.0)), Some(280.0));
        assert_eq!(clamped_width(None, Some(220.0)), None);
    }

    #[test]
    #[should_panic(expected = "Sidebar width must be a finite, non-negative number")]
    fn invalid_width_panics() {
        clamped_width(Some(f64::NAN), None);
    }
}

impl WindowContext {
    fn mount_sidebar(
        &self,
        owner: String,
        sidebar_host: &ContentHost,
        open: PropValue<Option<bool>>,
        on_open_change: PropValue<Option<Shared<dyn Fn(bool)>>>,
        width: PropValue<Option<f64>>,
        min_width: PropValue<Option<f64>>,
        resizable: PropValue<bool>,
    ) {
        assert!(
            self.sidebar.borrow().is_none(),
            "an AppKit Window can only contain one mounted Sidebar"
        );

        let mtm = MainThreadMarker::new().expect("Sidebar must be mounted on the main thread");
        let controller =
            SidebarSplitViewController::new(mtm, open, on_open_change, width, min_width, resizable);
        let sidebar_controller = NSViewController::new(mtm);
        sidebar_controller.setView(sidebar_host);
        let content_controller = NSViewController::new(mtm);
        content_controller.setView(&self.main_content_host);

        let sidebar_item = NSSplitViewItem::sidebarWithViewController(&sidebar_controller);
        let content_item = NSSplitViewItem::splitViewItemWithViewController(&content_controller);
        controller.addSplitViewItem(&sidebar_item);
        controller.addSplitViewItem(&content_item);
        controller.set_sidebar_item(sidebar_item);

        self.ns_window.setContentViewController(Some(&controller));
        controller.apply_sizing_props();
        self.sidebar
            .borrow_mut()
            .replace(MountedSidebar { owner, controller });
    }

    fn unmount_sidebar(&self, owner: &str) {
        let owns_sidebar = self
            .sidebar
            .borrow()
            .as_ref()
            .is_some_and(|mounted| mounted.owner == owner);
        if !owns_sidebar {
            return;
        }

        self.sidebar.borrow_mut().take();
        self.ns_window.setContentViewController(None);
        self.ns_window.setContentView(Some(&self.main_content_host));
        self.main_content_host.resize_tree();
    }

    fn set_sidebar_open(&self, owner: &str, open: Option<bool>) {
        let sidebar = self.sidebar.borrow();
        let Some(sidebar) = sidebar.as_ref().filter(|sidebar| sidebar.owner == owner) else {
            return;
        };
        sidebar.controller.apply_open_prop(open);
    }

    fn update_sidebar_sizing(&self, owner: &str) {
        let sidebar = self.sidebar.borrow();
        let Some(sidebar) = sidebar.as_ref().filter(|sidebar| sidebar.owner == owner) else {
            return;
        };
        sidebar.controller.apply_sizing_props();
    }
}

#[component]
pub fn Sidebar(props: &SidebarProps, element: &Element) -> Element {
    require_visual_mount!(element, Sidebar, output);

    let window = element
        .context::<WindowContext>()
        .expect("Sidebar must be mounted beneath an AppKit Window");
    let owner = nanoid::nanoid!();
    let tree_context = Rc::new(TreeContext::new());
    let mtm = MainThreadMarker::new().expect("Sidebar must be mounted on the main thread");
    let host = ContentHost::new(mtm, tree_context.clone());

    window.mount_sidebar(
        owner.clone(),
        &host,
        props.open.clone(),
        props.on_open_change.clone(),
        props.width.clone(),
        props.min_width.clone(),
        props.resizable.clone(),
    );

    element.on_unmount(closure!(
        [window, owner] || {
            window.unmount_sidebar(&owner);
        }
    ));

    scoped_effect!(
        [window, owner, props.open] || {
            window.set_sidebar_open(&owner, open.get());
        }
    );

    scoped_effect!(
        [window, owner, props.width, props.min_width, props.resizable] || {
            let _ = (width.get(), min_width.get(), resizable.get());
            window.update_sidebar_sizing(&owner);
        }
    );

    layout! {
        ContextProvider<TreeContext>(tree_context.clone()) {
            ContextProvider<ParentContext>(
                ParentContext {
                    add_child: Some(callback!([host] |object: &NSObject,
                    child_node: Option<NodeId> | {
                        let view = object.downcast_ref::<NSView>().unwrap();
                        host.set_child(view, child_node);
                    })),
                    insert_child: None,
                    remove_child: Some(callback!([host] |object: &NSObject,
                    _: Option<NodeId> | {
                        let view = object.downcast_ref::<NSView>().unwrap();
                        host.remove_child(view);
                    })),
                    parent_node: None
                },
            ) {
                $(props.children.clone().map(|child| Layout::from(child.clone())))
            }
        }
    }
}
