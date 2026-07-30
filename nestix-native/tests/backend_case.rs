use std::{cell::Cell, rc::Rc};

use nestix::{
    ContextProvider, Element, component, components::Fragment, layout, mount_root, props,
    unmount_root,
};
use nestix_native::{Backend, BackendCase, BackendContext, Root, RootProps};

struct TestBackend(&'static str);

impl Backend for TestBackend {
    fn backend_id(&self) -> &'static str {
        self.0
    }

    fn create_root(&self, props: RootProps) -> Option<Element> {
        Some(layout! {
            Fragment(.children = props.children.clone())
        })
    }
}

static MATCHING_BACKEND: TestBackend = TestBackend("matching");

#[props]
struct CounterProps {
    count: Rc<Cell<usize>>,
}

#[component]
fn Counter(props: &CounterProps) {
    let count = props.count.get();
    count.set(count.get() + 1);
}

#[props]
struct AppProps {
    default: Rc<Cell<usize>>,
    replacement: Rc<Cell<usize>>,
}

#[component]
fn App(props: &AppProps) -> Element {
    layout! {
        Root {
            BackendCase(
                "matching",
                .replacement = layout! {
                    Counter(.count = props.replacement.get())
                    Counter(.count = props.replacement.get())
                },
            ) {
                Counter(.count = props.default.get())
                Counter(.count = props.default.get())
            }
        }
    }
}

#[test]
fn renders_replacement_for_matching_backend_with_root_inside_parent_component() {
    let default = Rc::new(Cell::new(0));
    let replacement = Rc::new(Cell::new(0));
    let element = layout! {
        ContextProvider::<
        BackendContext
        >(Rc::new(BackendContext { backend: &MATCHING_BACKEND })) {
            App(.default = default.clone(), .replacement = replacement.clone())
        }
    };

    mount_root(&element);

    assert_eq!(default.get(), 0);
    assert_eq!(replacement.get(), 2);
    unmount_root().unwrap();
}

#[test]
fn renders_default_children_for_a_different_backend() {
    let default = Rc::new(Cell::new(0));
    let replacement = Rc::new(Cell::new(0));
    let element = layout! {
        ContextProvider::<
        BackendContext
        >(Rc::new(BackendContext { backend: &MATCHING_BACKEND })) {
            BackendCase(
                "different",
                .replacement = layout! {
                    Counter(.count = replacement.clone())
                    Counter(.count = replacement.clone())
                },
            ) {
                Counter(.count = default.clone())
                Counter(.count = default.clone())
            }
        }
    };

    mount_root(&element);

    assert_eq!(default.get(), 2);
    assert_eq!(replacement.get(), 0);
    unmount_root().unwrap();
}

#[test]
#[should_panic(expected = "BackendCase must be mounted beneath Root or BackendProvider")]
fn requires_backend_context() {
    mount_root(&layout! {
        BackendCase("matching")
    });
}
