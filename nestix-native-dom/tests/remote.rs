#![cfg(not(target_arch = "wasm32"))]

use std::{cell::RefCell, rc::Rc};

use nestix::{
    callback, components::ContextProvider, create_state, layout, mount_root, unmount_root,
};
use nestix_native_dom::{
    Button, DomDocumentRoot, DomRendererContext, DomSurfaceId, EmbeddedDomRuntime, FlexView, Input,
    Text,
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
    let value = create_state(String::new());
    let app = layout! {
        ContextProvider<DomRendererContext>(DomRendererContext::remote(runtime.clone())) {
            DomDocumentRoot {
                FlexView {
                    Text("Remote controls")
                    Button(
                        .title = "Remote",
                        .on_click = callback!([clicks] || clicks.update(|value| value + 1)),
                    )
                    Input(
                        .value = value.clone(),
                        .on_text_change = callback!(
                            [value] |next: &str| value.set(next.to_string())
                        ),
                    )
                }
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
    assert!(initial.contains(r#""tag":"div""#));
    assert!(initial.contains(r#""tag":"span""#));
    assert!(initial.contains(r#""tag":"input""#));
    assert!(initial.contains(r#""type":"replaceStyles""#));

    runtime
        .handle_message_json(r#"{"type":"event","node":3,"event":"click"}"#)
        .unwrap();
    assert_eq!(clicks.get(), 1);
    runtime
        .handle_message_json(r#"{"type":"event","node":4,"event":"input","value":"updated"}"#)
        .unwrap();
    assert_eq!(value.get(), "updated");

    unmount_root().unwrap();
}
