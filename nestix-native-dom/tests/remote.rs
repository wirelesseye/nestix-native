#![cfg(not(target_arch = "wasm32"))]

use std::{cell::RefCell, rc::Rc};

use nestix::{
    callback, components::ContextProvider, create_state, layout, mount_root, unmount_root,
};
use nestix_native_dom::{
    Button, DomDocumentRoot, DomRuntimeContext, DomSurfaceId, EmbeddedDomRuntime,
};

#[test]
fn remote_components_emit_commands_and_route_events() {
    let runtime = EmbeddedDomRuntime::new(DomSurfaceId(3));
    let batches = Rc::new(RefCell::new(Vec::<String>::new()));
    runtime.set_sender({
        let batches = batches.clone();
        move |json| batches.borrow_mut().push(json.to_string())
    });

    let clicks = create_state(0);
    let app = layout! {
        ContextProvider<DomRuntimeContext>(DomRuntimeContext { runtime: runtime.clone() }) {
            DomDocumentRoot {
                Button(
                    .title = "Remote",
                    .on_click = callback!([clicks] || clicks.update(|value| value + 1)),
                )
            }
        }
    };

    mount_root(&app);
    runtime.mark_ready();
    let initial = batches.borrow().join("");
    assert!(initial.contains(r#""type":"create""#));
    assert!(initial.contains(r#""type":"setText""#));
    assert!(initial.contains(r#""type":"listen""#));
    assert!(initial.contains(r#""type":"place""#));

    runtime
        .handle_message_json(r#"{"type":"event","node":1,"event":"click"}"#)
        .unwrap();
    assert_eq!(clicks.get(), 1);

    unmount_root().unwrap();
}
