use std::{cell::Cell, rc::Rc};

use nestix::{Element, component, components::Fragment, layout, mount_root, unmount_root};
use nestix_native::{Backend, BackendProvider, Button, ButtonProps, Root, RootProps};

struct TestBackend {
    id: &'static str,
    calls: Rc<Cell<usize>>,
    supports_button: bool,
}

#[component]
fn Empty() {}

impl Backend for TestBackend {
    fn backend_id(&self) -> &'static str {
        self.id
    }

    fn create_button(&self, _props: ButtonProps) -> Option<Element> {
        self.calls.set(self.calls.get() + 1);
        self.supports_button.then(|| layout! { Empty })
    }
}

fn backend(
    id: &'static str,
    calls: Rc<Cell<usize>>,
    supports_button: bool,
) -> &'static dyn Backend {
    Box::leak(Box::new(TestBackend {
        id,
        calls,
        supports_button,
    }))
}

struct RootBackend {
    id: &'static str,
    calls: Rc<Cell<usize>>,
    supports_root: bool,
}

impl Backend for RootBackend {
    fn backend_id(&self) -> &'static str {
        self.id
    }

    fn create_root(&self, props: RootProps) -> Option<Element> {
        self.calls.set(self.calls.get() + 1);
        self.supports_root.then(|| {
            layout! {
                Fragment(.children = props.children.clone())
            }
        })
    }
}

fn root_backend(
    id: &'static str,
    calls: Rc<Cell<usize>>,
    supports_root: bool,
) -> &'static dyn Backend {
    Box::leak(Box::new(RootBackend {
        id,
        calls,
        supports_root,
    }))
}

#[test]
fn nested_providers_fall_back_in_nearest_first_order() {
    let outer_calls = Rc::new(Cell::new(0));
    let inner_calls = Rc::new(Cell::new(0));
    let outer = backend("outer", outer_calls.clone(), true);
    let inner = backend("inner", inner_calls.clone(), false);

    let tree = layout! {
        BackendProvider(outer) {
            BackendProvider(inner) {
                Button(.title = "Fallback")
            }
        }
    };

    mount_root(&tree);
    assert_eq!(inner_calls.get(), 1);
    assert_eq!(outer_calls.get(), 1);
    unmount_root().unwrap();
}

#[test]
fn nearest_supporting_backend_stops_the_chain() {
    let outer_calls = Rc::new(Cell::new(0));
    let inner_calls = Rc::new(Cell::new(0));
    let outer = backend("outer", outer_calls.clone(), true);
    let inner = backend("inner", inner_calls.clone(), true);

    let tree = layout! {
        BackendProvider(outer) {
            BackendProvider(inner) {
                Button(.title = "Nearest")
            }
        }
    };

    mount_root(&tree);
    assert_eq!(inner_calls.get(), 1);
    assert_eq!(outer_calls.get(), 0);
    unmount_root().unwrap();
}

#[test]
fn root_uses_the_provider_fallback_chain() {
    let outer_calls = Rc::new(Cell::new(0));
    let inner_calls = Rc::new(Cell::new(0));
    let outer = root_backend("outer", outer_calls.clone(), true);
    let inner = root_backend("inner", inner_calls.clone(), false);

    let tree = layout! {
        BackendProvider(outer) {
            BackendProvider(inner) {
                Root {
                    Empty
                }
            }
        }
    };

    mount_root(&tree);
    assert_eq!(inner_calls.get(), 1);
    assert_eq!(outer_calls.get(), 1);
    unmount_root().unwrap();
}
