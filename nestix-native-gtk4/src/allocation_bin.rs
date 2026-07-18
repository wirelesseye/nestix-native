use std::cell::RefCell;

use gtk4::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct AllocationBin {
        pub child: RefCell<Option<gtk4::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AllocationBin {
        const NAME: &'static str = "NestixGtk4AllocationBin";
        type Type = super::AllocationBin;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for AllocationBin {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for AllocationBin {
        fn measure(&self, _: gtk4::Orientation, _: i32) -> (i32, i32, i32, i32) {
            // Taffy owns the child size. Do not let that size become a GTK
            // minimum or the window will be unable to shrink again.
            (0, 0, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if width > 0
                && height > 0
                && let Some(child) = self.child.borrow().as_ref()
            {
                child.allocate(width, height, baseline, None);
            }
        }
    }
}

glib::wrapper! {
    pub struct AllocationBin(ObjectSubclass<imp::AllocationBin>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl AllocationBin {
    pub(crate) fn new() -> Self {
        let bin: Self = glib::Object::new();
        bin.set_hexpand(true);
        bin.set_vexpand(true);
        bin
    }

    pub(crate) fn child(&self) -> Option<gtk4::Widget> {
        self.imp().child.borrow().clone()
    }

    pub(crate) fn set_child(&self, child: Option<&gtk4::Widget>) {
        if self.child().as_ref() == child {
            return;
        }

        if let Some(previous) = self.imp().child.borrow_mut().take() {
            previous.unparent();
        }
        if let Some(child) = child {
            child.set_parent(self);
            self.imp().child.replace(Some(child.clone()));
        }
    }
}
