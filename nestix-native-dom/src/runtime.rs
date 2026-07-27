use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use nestix::Shared;
use serde::{Deserialize, Serialize};

/// Identifier for one managed DOM document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomSurfaceId(pub u64);

/// Identifier for a node within one managed DOM document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomNodeId(pub u64);

/// Nestix host handle used by the DOM placement algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomNodeHandle {
    pub surface: DomSurfaceId,
    pub node: DomNodeId,
}

/// Serializable value assigned to a DOM property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DomValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

impl From<bool> for DomValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for DomValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for DomValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

/// A CSS declaration sent to a DOM runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomStyle {
    pub property: String,
    pub value: String,
}

impl DomStyle {
    pub fn new(property: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            property: property.into(),
            value: value.into(),
        }
    }
}

/// Mutation understood by the managed-document JavaScript bootstrap.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DomCommand {
    Create {
        node: DomNodeId,
        tag: String,
    },
    SetText {
        node: DomNodeId,
        value: String,
    },
    ReplaceStyles {
        node: DomNodeId,
        styles: Vec<DomStyle>,
    },
    SetAttribute {
        node: DomNodeId,
        name: String,
        value: Option<String>,
    },
    SetProperty {
        node: DomNodeId,
        name: String,
        value: DomValue,
    },
    Place {
        node: DomNodeId,
        parent: DomNodeId,
        predecessor: Option<DomNodeId>,
    },
    Listen {
        node: DomNodeId,
        event: String,
    },
    Remove {
        node: DomNodeId,
    },
}

/// Browser event forwarded by an embedded managed document.
#[derive(Debug, Clone, Deserialize)]
pub struct DomEventData {
    pub node: DomNodeId,
    pub event: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub checked: Option<bool>,
}

type Sender = Rc<dyn Fn(&str)>;
type Listener = Shared<dyn Fn(&DomEventData)>;

/// Per-surface runtime used by DOM components on native targets.
pub struct EmbeddedDomRuntime {
    surface: DomSurfaceId,
    next_node: Cell<u64>,
    ready: Cell<bool>,
    pending: RefCell<Vec<DomCommand>>,
    sender: RefCell<Option<Sender>>,
    listeners: RefCell<HashMap<(DomNodeId, String), Listener>>,
}

impl EmbeddedDomRuntime {
    pub fn new(surface: DomSurfaceId) -> Rc<Self> {
        Rc::new(Self {
            surface,
            next_node: Cell::new(1),
            ready: Cell::new(false),
            pending: RefCell::new(Vec::new()),
            sender: RefCell::new(None),
            listeners: RefCell::new(HashMap::new()),
        })
    }

    pub fn surface(&self) -> DomSurfaceId {
        self.surface
    }

    pub fn root_handle(&self) -> DomNodeHandle {
        DomNodeHandle {
            surface: self.surface,
            node: DomNodeId(0),
        }
    }

    pub fn create_element(&self, tag: impl Into<String>) -> DomNodeHandle {
        let node = DomNodeId(self.next_node.get());
        self.next_node.set(node.0 + 1);
        self.push(DomCommand::Create {
            node,
            tag: tag.into(),
        });
        DomNodeHandle {
            surface: self.surface,
            node,
        }
    }

    pub fn set_text(&self, node: DomNodeHandle, value: impl Into<String>) {
        self.assert_surface(node);
        self.push(DomCommand::SetText {
            node: node.node,
            value: value.into(),
        });
    }

    pub fn replace_styles(&self, node: DomNodeHandle, styles: Vec<DomStyle>) {
        self.assert_surface(node);
        self.push(DomCommand::ReplaceStyles {
            node: node.node,
            styles,
        });
    }

    pub fn set_attribute(
        &self,
        node: DomNodeHandle,
        name: impl Into<String>,
        value: Option<String>,
    ) {
        self.assert_surface(node);
        self.push(DomCommand::SetAttribute {
            node: node.node,
            name: name.into(),
            value,
        });
    }

    pub fn set_property(
        &self,
        node: DomNodeHandle,
        name: impl Into<String>,
        value: impl Into<DomValue>,
    ) {
        self.assert_surface(node);
        self.push(DomCommand::SetProperty {
            node: node.node,
            name: name.into(),
            value: value.into(),
        });
    }

    pub fn place(
        &self,
        node: DomNodeHandle,
        parent: DomNodeHandle,
        predecessor: Option<DomNodeHandle>,
    ) {
        self.assert_surface(node);
        self.assert_surface(parent);
        if let Some(predecessor) = predecessor {
            self.assert_surface(predecessor);
        }
        self.push(DomCommand::Place {
            node: node.node,
            parent: parent.node,
            predecessor: predecessor.map(|handle| handle.node),
        });
    }

    pub fn listen(&self, node: DomNodeHandle, event: impl Into<String>, listener: Listener) {
        self.assert_surface(node);
        let event = event.into();
        self.listeners
            .borrow_mut()
            .insert((node.node, event.clone()), listener);
        self.push(DomCommand::Listen {
            node: node.node,
            event,
        });
    }

    pub fn remove(&self, node: DomNodeHandle) {
        self.assert_surface(node);
        self.listeners
            .borrow_mut()
            .retain(|(listener_node, _), _| *listener_node != node.node);
        self.push(DomCommand::Remove { node: node.node });
    }

    pub fn set_sender(&self, sender: impl Fn(&str) + 'static) {
        self.sender.replace(Some(Rc::new(sender)));
        self.flush();
    }

    pub fn clear_sender(&self) {
        self.sender.take();
        self.ready.set(false);
    }

    pub fn mark_ready(&self) {
        self.ready.set(true);
        self.flush();
    }

    pub fn dispatch_event_json(&self, json: &str) -> Result<(), serde_json::Error> {
        let event: DomEventData = serde_json::from_str(json)?;
        let listener = self
            .listeners
            .borrow()
            .get(&(event.node, event.event.clone()))
            .cloned();
        if let Some(listener) = listener {
            listener(&event);
        }
        Ok(())
    }

    pub fn handle_message_json(&self, json: &str) -> Result<(), serde_json::Error> {
        let message: serde_json::Value = serde_json::from_str(json)?;
        if message.get("type").and_then(serde_json::Value::as_str) == Some("ready") {
            self.mark_ready();
            Ok(())
        } else {
            self.dispatch_event_json(json)
        }
    }

    fn push(&self, command: DomCommand) {
        self.pending.borrow_mut().push(command);
        self.flush();
    }

    fn flush(&self) {
        if !self.ready.get() {
            return;
        }
        let Some(sender) = self.sender.borrow().clone() else {
            return;
        };
        let commands = self.pending.take();
        if commands.is_empty() {
            return;
        }
        let json = serde_json::to_string(&commands).expect("DOM commands must be serializable");
        sender(&json);
    }

    fn assert_surface(&self, handle: DomNodeHandle) {
        assert_eq!(
            handle.surface, self.surface,
            "cannot mix DOM nodes belonging to different surfaces"
        );
    }
}

/// Context supplied by a native managed DOM surface.
#[derive(Clone)]
pub struct DomRuntimeContext {
    pub runtime: Rc<EmbeddedDomRuntime>,
}

/// HTML loaded by native managed DOM surfaces.
pub const DOM_BOOTSTRAP_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<style>html,body,#nestix-root{box-sizing:border-box;width:100%;height:100%;margin:0;background-color:transparent}body{overflow:auto}</style></head>
<body><div id="nestix-root"></div><script>
(() => {
  const nodes = new Map([[0, document.getElementById('nestix-root')]]);
  const post = value => window.webkit.messageHandlers.nestix.postMessage(JSON.stringify(value));
  window.__nestixApply = commands => {
    for (const command of commands) {
      const node = nodes.get(command.node);
      switch (command.type) {
        case 'create': nodes.set(command.node, document.createElement(command.tag)); break;
        case 'setText': node.textContent = command.value; break;
        case 'replaceStyles':
          node.removeAttribute('style');
          for (const style of command.styles) node.style.setProperty(style.property, style.value);
          break;
        case 'setAttribute':
          if (command.value === null) node.removeAttribute(command.name);
          else node.setAttribute(command.name, command.value);
          break;
        case 'setProperty': node[command.name] = command.value; break;
        case 'place': {
          const parent = nodes.get(command.parent);
          const predecessor = command.predecessor == null ? null : nodes.get(command.predecessor);
          parent.insertBefore(node, predecessor ? predecessor.nextSibling : parent.firstChild);
          break;
        }
        case 'listen':
          node.addEventListener(command.event, event => post({
            type: 'event', node: command.node, event: command.event,
            value: event.target && 'value' in event.target ? event.target.value : null,
            checked: event.target && 'checked' in event.target ? event.target.checked : null
          }));
          break;
        case 'remove': node.remove(); nodes.delete(command.node); break;
      }
    }
  };
  post({type: 'ready'});
})();
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use nestix::Shared;

    use super::*;

    #[test]
    fn queues_until_ready_and_dispatches_events() {
        let runtime = EmbeddedDomRuntime::new(DomSurfaceId(9));
        let batches = Rc::new(RefCell::new(Vec::<String>::new()));
        runtime.set_sender({
            let batches = batches.clone();
            move |json| batches.borrow_mut().push(json.to_string())
        });

        let button = runtime.create_element("button");
        runtime.set_text(button, "Save");
        runtime.place(button, runtime.root_handle(), None);
        assert!(batches.borrow().is_empty());

        runtime.mark_ready();
        let commands: serde_json::Value =
            serde_json::from_str(&batches.borrow()[0]).expect("valid command batch");
        assert_eq!(commands.as_array().unwrap().len(), 3);

        let clicks = Rc::new(Cell::new(0));
        runtime.listen(
            button,
            "click",
            Shared::from(Rc::new({
                let clicks = clicks.clone();
                move |_: &DomEventData| clicks.set(clicks.get() + 1)
            }) as Rc<dyn Fn(&DomEventData)>),
        );
        runtime
            .handle_message_json(r#"{"type":"event","node":1,"event":"click"}"#)
            .unwrap();
        assert_eq!(clicks.get(), 1);
    }
}
