use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use js_sys::{Object, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Event, HtmlElement, Node};

use crate::{
    DomEventData, DomNodeHandle, DomNodeId, DomStyle, DomSurfaceId, DomValue,
    renderer::{DomEventListener, DomRenderer},
};

struct BrowserListener {
    event: String,
    callback: Closure<dyn FnMut(Event)>,
}

pub(crate) struct BrowserDomRenderer {
    next_node: Cell<u64>,
    nodes: RefCell<HashMap<DomNodeId, Node>>,
    listeners: RefCell<HashMap<DomNodeId, Vec<BrowserListener>>>,
}

impl BrowserDomRenderer {
    pub(crate) fn new(root: Node) -> Rc<Self> {
        Rc::new(Self {
            next_node: Cell::new(1),
            nodes: RefCell::new(HashMap::from([(DomNodeId(0), root)])),
            listeners: RefCell::new(HashMap::new()),
        })
    }

    fn node(&self, handle: DomNodeHandle) -> Node {
        assert_eq!(handle.surface, DomSurfaceId(0));
        self.nodes
            .borrow()
            .get(&handle.node)
            .cloned()
            .unwrap_or_else(|| panic!("unknown browser DOM node {:?}", handle.node))
    }

    fn element(&self, handle: DomNodeHandle) -> HtmlElement {
        self.node(handle)
            .dyn_into()
            .expect("managed browser DOM node must be an HtmlElement")
    }

    fn clear_listeners(&self, node: DomNodeId) {
        let Some(listeners) = self.listeners.borrow_mut().remove(&node) else {
            return;
        };
        let target = self
            .nodes
            .borrow()
            .get(&node)
            .cloned()
            .expect("listener node must remain mounted");
        for listener in listeners {
            target
                .remove_event_listener_with_callback(
                    &listener.event,
                    listener.callback.as_ref().unchecked_ref(),
                )
                .expect("failed to remove DOM event listener");
        }
    }
}

impl DomRenderer for BrowserDomRenderer {
    fn scale_factor(&self) -> f64 {
        web_sys::window().map_or(1.0, |window| window.device_pixel_ratio())
    }

    fn root_handle(&self) -> DomNodeHandle {
        DomNodeHandle {
            surface: DomSurfaceId(0),
            node: DomNodeId(0),
        }
    }

    fn create_element(&self, tag: &str) -> DomNodeHandle {
        let node = crate::dom::create_html_element(tag).unchecked_into::<Node>();
        let id = DomNodeId(self.next_node.get());
        self.next_node.set(id.0 + 1);
        self.nodes.borrow_mut().insert(id, node);
        DomNodeHandle {
            surface: DomSurfaceId(0),
            node: id,
        }
    }

    fn set_text(&self, node: DomNodeHandle, value: String) {
        self.node(node).set_text_content(Some(&value));
    }

    fn replace_styles(&self, node: DomNodeHandle, styles: Vec<DomStyle>) {
        let css = self.element(node).style();
        css.set_css_text("");
        for style in styles {
            css.set_property(&style.property, &style.value)
                .unwrap_or_else(|_| panic!("failed to set CSS property `{}`", style.property));
        }
    }

    fn set_attribute(&self, node: DomNodeHandle, name: String, value: Option<String>) {
        let element = self.element(node);
        match value {
            Some(value) => element
                .set_attribute(&name, &value)
                .unwrap_or_else(|_| panic!("failed to set DOM attribute `{name}`")),
            None => element
                .remove_attribute(&name)
                .unwrap_or_else(|_| panic!("failed to remove DOM attribute `{name}`")),
        }
    }

    fn set_property(&self, node: DomNodeHandle, name: String, value: DomValue) {
        let value = match value {
            DomValue::Null => JsValue::NULL,
            DomValue::Bool(value) => JsValue::from_bool(value),
            DomValue::Number(value) => JsValue::from_f64(value),
            DomValue::String(value) => JsValue::from_str(&value),
        };
        let node = self.node(node);
        let key = JsValue::from_str(&name);
        if Reflect::get(node.as_ref(), &key).is_ok_and(|current| Object::is(&current, &value)) {
            return;
        }
        Reflect::set(node.as_ref(), &key, &value)
            .unwrap_or_else(|_| panic!("failed to set DOM property `{name}`"));
    }

    fn place(
        &self,
        node: DomNodeHandle,
        parent: DomNodeHandle,
        predecessor: Option<DomNodeHandle>,
    ) {
        let node = self.node(node);
        let parent = self.node(parent);
        if let Some(predecessor) = predecessor {
            let predecessor = self.node(predecessor);
            parent
                .insert_before(&node, predecessor.next_sibling().as_ref())
                .expect("failed to place DOM node after its predecessor");
        } else {
            parent
                .insert_before(&node, parent.first_child().as_ref())
                .expect("failed to place first DOM child");
        }
    }

    fn listen(&self, node: DomNodeHandle, event: String, listener: DomEventListener) {
        let target = self.node(node);
        let event_name = event.clone();
        let callback = Closure::<dyn FnMut(Event)>::new(move |browser_event: Event| {
            let target = browser_event.target();
            let value = target.as_ref().and_then(|target| {
                Reflect::get(target, &JsValue::from_str("value"))
                    .ok()
                    .and_then(|value| value.as_string())
            });
            let checked = target.as_ref().and_then(|target| {
                Reflect::get(target, &JsValue::from_str("checked"))
                    .ok()
                    .and_then(|value| value.as_bool())
            });
            listener(&DomEventData {
                node: node.node,
                event: event_name.clone(),
                value,
                checked,
            });
        });
        target
            .add_event_listener_with_callback(&event, callback.as_ref().unchecked_ref())
            .expect("failed to add DOM event listener");
        self.listeners
            .borrow_mut()
            .entry(node.node)
            .or_default()
            .push(BrowserListener { event, callback });
    }

    fn remove(&self, node: DomNodeHandle) {
        self.clear_listeners(node.node);
        if let Some(node) = self.nodes.borrow_mut().remove(&node.node)
            && let Some(parent) = node.parent_node()
        {
            parent
                .remove_child(&node)
                .expect("failed to remove DOM node during unmount");
        }
    }

    fn html_element(&self, node: DomNodeHandle) -> HtmlElement {
        self.element(node)
    }
}
