use std::{cell::RefCell, collections::HashSet, fmt, rc::Rc};

use nestix::{Element, Layout, Shared, component, layout, props};
use nestix_native_core::{
    ClassList, StyleContext, StyleScope, ViewProps, matched_style, resolved_view_style,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
#[cfg(target_arch = "wasm32")]
use web_sys::{AddEventListenerOptions, Event};

#[cfg(target_arch = "wasm32")]
use crate::renderer::DomRenderer;
use crate::{
    DomEventData, DomEventOptions, DomNodeHandle, DomValue,
    renderer::{mount_host, renderer},
    style_declarations::{padding_styles, view_styles},
};

/// A reactive HTML attribute value.
#[derive(Debug, Clone)]
pub enum DomAttributeValue {
    /// A string attribute. `None` removes the attribute.
    String(Option<String>),
    /// A boolean attribute represented by its presence or absence.
    Boolean(bool),
}

/// An attribute applied to a [`DomElement`].
#[derive(Debug, Clone)]
pub struct DomAttribute {
    name: String,
    value: DomAttributeValue,
}

impl DomAttribute {
    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::optional_string(name, Some(value.into()))
    }

    pub fn optional_string(name: impl Into<String>, value: Option<String>) -> Self {
        Self {
            name: name.into(),
            value: DomAttributeValue::String(value),
        }
    }

    pub fn boolean(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: name.into(),
            value: DomAttributeValue::Boolean(value),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &DomAttributeValue {
        &self.value
    }
}

/// Value assigned to a [`DomElement`] property.
#[derive(Debug, Clone)]
pub enum DomPropertyValue {
    Portable(DomValue),
    #[cfg(target_arch = "wasm32")]
    JavaScript(JsValue),
}

/// A property applied to a [`DomElement`].
#[derive(Debug, Clone)]
pub struct DomProperty {
    name: String,
    value: DomPropertyValue,
}

impl DomProperty {
    /// Creates a property supported by browser and native DOM renderers.
    pub fn new(name: impl Into<String>, value: impl Into<DomValue>) -> Self {
        Self {
            name: name.into(),
            value: DomPropertyValue::Portable(value.into()),
        }
    }

    /// Creates a browser-only property containing an arbitrary JavaScript value.
    #[cfg(target_arch = "wasm32")]
    pub fn javascript(name: impl Into<String>, value: impl Into<JsValue>) -> Self {
        Self {
            name: name.into(),
            value: DomPropertyValue::JavaScript(value.into()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &DomPropertyValue {
        &self.value
    }
}

#[derive(Clone)]
enum DomEventHandler {
    Portable(Shared<dyn Fn(&DomEventData)>),
    #[cfg(target_arch = "wasm32")]
    Browser(Shared<dyn Fn(&Event)>),
}

/// A DOM event listener attached for the lifetime of a [`DomElement`].
#[derive(Clone)]
pub struct DomEvent {
    name: String,
    handler: DomEventHandler,
    options: DomEventOptions,
}

impl fmt::Debug for DomEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DomEvent")
            .field("name", &self.name)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl DomEvent {
    /// Creates an event supported by browser and native DOM renderers.
    pub fn new(name: impl Into<String>, handler: impl Fn(&DomEventData) + 'static) -> Self {
        Self {
            name: name.into(),
            handler: DomEventHandler::Portable(Shared::from(
                Rc::new(handler) as Rc<dyn Fn(&DomEventData)>
            )),
            options: DomEventOptions::default(),
        }
    }

    /// Creates a browser-only listener that receives the raw web event.
    #[cfg(target_arch = "wasm32")]
    pub fn browser(name: impl Into<String>, handler: impl Fn(&Event) + 'static) -> Self {
        Self {
            name: name.into(),
            handler: DomEventHandler::Browser(Shared::from(Rc::new(handler) as Rc<dyn Fn(&Event)>)),
            options: DomEventOptions::default(),
        }
    }

    pub fn capture(mut self, capture: bool) -> Self {
        self.options.capture = capture;
        self
    }

    pub fn once(mut self, once: bool) -> Self {
        self.options.once = once;
        self
    }

    pub fn passive(mut self, passive: bool) -> Self {
        self.options.passive = passive;
        self
    }
}

#[derive(Clone)]
struct MountedDomElement {
    #[cfg(target_arch = "wasm32")]
    renderer: Rc<dyn DomRenderer>,
    node: DomNodeHandle,
}

/// A shared reference populated while a [`DomElement`] is mounted.
#[derive(Clone, Default)]
pub struct DomElementRef(Rc<RefCell<Option<MountedDomElement>>>);

impl DomElementRef {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the portable node handle while the element is mounted.
    pub fn handle(&self) -> Option<DomNodeHandle> {
        self.0.borrow().as_ref().map(|mounted| mounted.node)
    }

    /// Returns the browser element while mounted in a browser renderer.
    #[cfg(target_arch = "wasm32")]
    pub fn get(&self) -> Option<web_sys::Element> {
        self.0
            .borrow()
            .as_ref()
            .map(|mounted| mounted.renderer.html_element(mounted.node).unchecked_into())
    }

    fn set(&self, mounted: Option<MountedDomElement>) {
        self.0.replace(mounted);
    }
}

impl fmt::Debug for DomElementRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DomElementRef")
            .field("mounted", &self.0.borrow().is_some())
            .finish()
    }
}

/// Properties for [`DomElement`].
#[props]
#[derive(Clone)]
pub struct DomElementProps {
    #[props(start)]
    pub tag: String,

    #[props(default)]
    pub class: ClassList,

    #[props(default)]
    pub dom_class: String,

    #[props(nested, default)]
    pub view: ViewProps,

    #[props(default)]
    pub attributes: Vec<DomAttribute>,

    #[props(default)]
    pub properties: Vec<DomProperty>,

    #[props(raw, default)]
    pub events: Vec<DomEvent>,

    #[props(raw, default)]
    pub node_ref: Option<DomElementRef>,

    #[props(default)]
    pub children: Layout,
}

/// Renders an arbitrary HTML or registered custom element.
#[component]
pub fn DomElement(props: &DomElementProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__DomElement", "__dom_DomElement"];

    let renderer = renderer(element);
    let node = renderer.create_element(&props.tag.get());
    mount_host(element, renderer.clone(), node);

    if let Some(node_ref) = &props.node_ref {
        node_ref.set(Some(MountedDomElement {
            #[cfg(target_arch = "wasm32")]
            renderer: renderer.clone(),
            node,
        }));
        element.on_unmount({
            let node_ref = node_ref.clone();
            move || node_ref.set(None)
        });
    }

    let matched = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let effective_style = resolved_view_style(matched, &props.view);
    element.scoped_effect({
        let renderer = renderer.clone();
        let effective_style = effective_style.clone();
        let dom_class = props.dom_class.clone();
        move || {
            let style = effective_style.get().unwrap_or_default();
            let scale_factor = renderer.scale_factor();
            let mut styles = view_styles(&style, scale_factor);
            styles.extend(padding_styles(&style, scale_factor));
            renderer.replace_styles(node, styles);
            let class = dom_class.get();
            renderer.set_attribute(
                node,
                "class".to_string(),
                (!class.is_empty()).then_some(class),
            );
        }
    });

    let previous_attributes = Rc::new(RefCell::new(HashSet::<String>::new()));
    element.scoped_effect({
        let renderer = renderer.clone();
        let attributes = props.attributes.clone();
        let previous_attributes = previous_attributes.clone();
        move || {
            let attributes = attributes.get();
            let current = validate_names(
                attributes.iter().map(|attribute| attribute.name()),
                "attribute",
            );
            for name in previous_attributes.borrow().difference(&current) {
                renderer.set_attribute(node, name.clone(), None);
            }
            for attribute in attributes {
                assert!(
                    attribute.name != "class" && attribute.name != "style",
                    "use `dom_class` and Nestix styles instead of the reserved `{}` attribute",
                    attribute.name
                );
                let value = match attribute.value {
                    DomAttributeValue::String(value) => value,
                    DomAttributeValue::Boolean(false) => None,
                    DomAttributeValue::Boolean(true) => Some(String::new()),
                };
                renderer.set_attribute(node, attribute.name, value);
            }
            previous_attributes.replace(current);
        }
    });

    let previous_properties = Rc::new(RefCell::new(HashSet::<String>::new()));
    element.scoped_effect({
        let renderer = renderer.clone();
        let properties = props.properties.clone();
        let previous_properties = previous_properties.clone();
        move || {
            let properties = properties.get();
            let current = validate_names(
                properties.iter().map(|property| property.name()),
                "property",
            );
            for name in previous_properties.borrow().difference(&current) {
                renderer.remove_property(node, name.clone());
            }
            for property in properties {
                match property.value {
                    DomPropertyValue::Portable(value) => {
                        renderer.set_property(node, property.name, value)
                    }
                    #[cfg(target_arch = "wasm32")]
                    DomPropertyValue::JavaScript(value) => {
                        let html = renderer.html_element(node);
                        js_sys::Reflect::set(
                            html.as_ref(),
                            &JsValue::from_str(&property.name),
                            &value,
                        )
                        .unwrap_or_else(|_| {
                            panic!("failed to set DOM property `{}`", property.name)
                        });
                    }
                }
            }
            previous_properties.replace(current);
        }
    });

    let mut event_names = HashSet::new();
    #[cfg(target_arch = "wasm32")]
    let mut browser_listeners = Vec::new();
    for event in &props.events {
        assert!(!event.name.is_empty(), "DOM event name cannot be empty");
        assert!(
            event_names.insert(event.name.clone()),
            "duplicate DOM event listener `{}`",
            event.name
        );
        match &event.handler {
            DomEventHandler::Portable(handler) => {
                renderer.listen(node, event.name.clone(), event.options, handler.clone())
            }
            #[cfg(target_arch = "wasm32")]
            DomEventHandler::Browser(handler) => {
                let html = renderer.html_element(node);
                let handler = handler.clone();
                let callback =
                    Closure::<dyn FnMut(Event)>::new(move |event: Event| handler(&event));
                let options = AddEventListenerOptions::new();
                options.set_capture(event.options.capture);
                options.set_once(event.options.once);
                options.set_passive(event.options.passive);
                html.add_event_listener_with_callback_and_add_event_listener_options(
                    &event.name,
                    callback.as_ref().unchecked_ref(),
                    &options,
                )
                .expect("failed to add browser DOM event listener");
                browser_listeners.push((html, event.name.clone(), event.options.capture, callback));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    element.on_unmount(move || {
        for (html, name, capture, callback) in &browser_listeners {
            html.remove_event_listener_with_callback_and_bool(
                name,
                callback.as_ref().unchecked_ref(),
                *capture,
            )
            .expect("failed to remove browser DOM event listener");
        }
    });

    layout! {
        StyleScope(
            .class = props.class.clone(),
            .default_classes = DEFAULT_CLASSES,
            .effective_style = effective_style,
        ) {
            $(props.children.clone())
        }
    }
}

fn validate_names<'a>(names: impl IntoIterator<Item = &'a str>, kind: &str) -> HashSet<String> {
    let mut unique = HashSet::new();
    for name in names {
        assert!(!name.is_empty(), "DOM {kind} name cannot be empty");
        assert!(
            unique.insert(name.to_string()),
            "duplicate DOM {kind} `{name}`"
        );
    }
    unique
}
