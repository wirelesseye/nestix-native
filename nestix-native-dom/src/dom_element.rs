use std::{cell::RefCell, collections::HashSet, fmt, rc::Rc};

use nestix::{Element, Layout, Shared, component, layout, props};
use nestix_native_core::{
    ClassList, StyleContext, StyleScope, ViewProps, matched_style, resolved_view_style,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{AddEventListenerOptions, Event, Node};

use crate::{
    dom::{create_html_element, mount_host},
    style::{apply_padding, apply_view_style},
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
    /// Creates a string-valued attribute.
    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::optional_string(name, Some(value.into()))
    }

    /// Creates an optional string-valued attribute.
    pub fn optional_string(name: impl Into<String>, value: Option<String>) -> Self {
        Self {
            name: name.into(),
            value: DomAttributeValue::String(value),
        }
    }

    /// Creates a boolean presence attribute.
    pub fn boolean(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: name.into(),
            value: DomAttributeValue::Boolean(value),
        }
    }

    /// Returns the attribute name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute value.
    pub fn value(&self) -> &DomAttributeValue {
        &self.value
    }
}

/// A JavaScript property applied to a [`DomElement`].
#[derive(Debug, Clone)]
pub struct DomProperty {
    name: String,
    value: JsValue,
}

impl DomProperty {
    /// Creates a JavaScript property assignment.
    pub fn new(name: impl Into<String>, value: impl Into<JsValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the property name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the property value.
    pub fn value(&self) -> &JsValue {
        &self.value
    }
}

/// Registration options for a DOM event listener.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DomEventOptions {
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
}

/// A DOM event listener attached for the lifetime of a [`DomElement`].
#[derive(Clone)]
pub struct DomEvent {
    name: String,
    handler: Shared<dyn Fn(&Event)>,
    options: DomEventOptions,
}

impl fmt::Debug for DomEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DomEvent")
            .field("name", &self.name)
            .field("handler", &self.handler)
            .field("options", &self.options)
            .finish()
    }
}

impl DomEvent {
    /// Creates a listener for `name`.
    pub fn new(name: impl Into<String>, handler: impl Fn(&Event) + 'static) -> Self {
        Self {
            name: name.into(),
            handler: Shared::from(Rc::new(handler) as Rc<dyn Fn(&Event)>),
            options: DomEventOptions::default(),
        }
    }

    /// Sets whether the listener runs during event capture.
    pub fn capture(mut self, capture: bool) -> Self {
        self.options.capture = capture;
        self
    }

    /// Sets whether the browser removes the listener after its first event.
    pub fn once(mut self, once: bool) -> Self {
        self.options.once = once;
        self
    }

    /// Sets whether the listener promises not to cancel the event.
    pub fn passive(mut self, passive: bool) -> Self {
        self.options.passive = passive;
        self
    }
}

/// A shared reference populated while a [`DomElement`] is mounted.
#[derive(Clone, Default)]
pub struct DomElementRef(Rc<RefCell<Option<web_sys::Element>>>);

impl DomElementRef {
    /// Creates an empty element reference.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the mounted element, if it is currently available.
    pub fn get(&self) -> Option<web_sys::Element> {
        self.0.borrow().clone()
    }

    fn set(&self, element: Option<web_sys::Element>) {
        self.0.replace(element);
    }
}

impl fmt::Debug for DomElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DomElementRef")
            .field("mounted", &self.0.borrow().is_some())
            .finish()
    }
}

/// Properties for [`DomElement`].
#[props]
#[derive(Clone)]
pub struct DomElementProps {
    /// The immutable HTML tag name.
    #[props(start)]
    pub tag: String,

    /// Classes used by Nestix style matching. These are not emitted to the DOM.
    #[props(default)]
    pub class: ClassList,

    /// The actual HTML `class` attribute emitted to the DOM.
    #[props(default)]
    pub dom_class: String,

    /// Common Nestix Native layout properties.
    #[props(nested, default)]
    pub view: ViewProps,

    /// Reactive HTML attributes. Replacing the vector reconciles removals.
    #[props(default)]
    pub attributes: Vec<DomAttribute>,

    /// Reactive JavaScript properties. Replacing the vector deletes removals.
    #[props(default)]
    pub properties: Vec<DomProperty>,

    /// Event listeners installed for the lifetime of this element.
    #[props(raw, default)]
    pub events: Vec<DomEvent>,

    /// Optional reference populated while this element is mounted.
    #[props(raw, default)]
    pub node_ref: Option<DomElementRef>,

    /// Components mounted into the element's light DOM.
    #[props(default)]
    pub children: Layout,
}

/// Renders an arbitrary HTML or custom element.
#[component]
pub fn DomElement(props: &DomElementProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__DomElement", "__dom_DomElement"];

    let html = create_html_element(&props.tag.get());
    let node = html.clone().unchecked_into::<Node>();
    mount_host(element, &node);

    if let Some(node_ref) = &props.node_ref {
        node_ref.set(Some(html.clone().unchecked_into()));
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
        let html = html.clone();
        let effective_style = effective_style.clone();
        let dom_class = props.dom_class.clone();
        move || {
            let style = effective_style.get().unwrap_or_default();
            html.set_class_name(&dom_class.get());
            apply_view_style(&html.style(), &style);
            apply_padding(&html.style(), &style);
        }
    });

    let previous_attributes = Rc::new(RefCell::new(HashSet::<String>::new()));
    element.scoped_effect({
        let html = html.clone();
        let attributes = props.attributes.clone();
        let previous_attributes = previous_attributes.clone();
        move || {
            let attributes = attributes.get();
            let current = validate_names(
                attributes.iter().map(|attribute| attribute.name()),
                "attribute",
            );
            for name in previous_attributes.borrow().difference(&current) {
                html.remove_attribute(name)
                    .expect("failed to remove DOM attribute");
            }
            for attribute in attributes {
                assert!(
                    attribute.name != "class" && attribute.name != "style",
                    "use `dom_class` and Nestix styles instead of the reserved `{}` attribute",
                    attribute.name
                );
                match attribute.value {
                    DomAttributeValue::String(Some(value)) => html
                        .set_attribute(&attribute.name, &value)
                        .expect("failed to set DOM attribute"),
                    DomAttributeValue::String(None) | DomAttributeValue::Boolean(false) => html
                        .remove_attribute(&attribute.name)
                        .expect("failed to remove DOM attribute"),
                    DomAttributeValue::Boolean(true) => html
                        .set_attribute(&attribute.name, "")
                        .expect("failed to set boolean DOM attribute"),
                }
            }
            previous_attributes.replace(current);
        }
    });

    let previous_properties = Rc::new(RefCell::new(HashSet::<String>::new()));
    element.scoped_effect({
        let html = html.clone();
        let properties = props.properties.clone();
        let previous_properties = previous_properties.clone();
        move || {
            let properties = properties.get();
            let current = validate_names(
                properties.iter().map(|property| property.name()),
                "property",
            );
            for name in previous_properties.borrow().difference(&current) {
                js_sys::Reflect::delete_property(html.as_ref(), &JsValue::from_str(name))
                    .expect("failed to delete DOM property");
            }
            for property in properties {
                js_sys::Reflect::set(
                    html.as_ref(),
                    &JsValue::from_str(&property.name),
                    &property.value,
                )
                .expect("failed to set DOM property");
            }
            previous_properties.replace(current);
        }
    });

    let mut event_names = HashSet::new();
    let mut listeners = Vec::new();
    for event in &props.events {
        assert!(!event.name.is_empty(), "DOM event name cannot be empty");
        assert!(
            event_names.insert(event.name.clone()),
            "duplicate DOM event listener `{}`",
            event.name
        );
        let handler = event.handler.clone();
        let callback = Closure::<dyn FnMut(Event)>::new(move |event| handler(&event));
        let options = AddEventListenerOptions::new();
        options.set_capture(event.options.capture);
        options.set_once(event.options.once);
        options.set_passive(event.options.passive);
        html.add_event_listener_with_callback_and_add_event_listener_options(
            &event.name,
            callback.as_ref().unchecked_ref(),
            &options,
        )
        .expect("failed to add DOM event listener");
        listeners.push((event.name.clone(), event.options.capture, callback));
    }
    element.on_unmount({
        let html = html.clone();
        move || {
            for (name, capture, callback) in &listeners {
                html.remove_event_listener_with_callback_and_bool(
                    name,
                    callback.as_ref().unchecked_ref(),
                    *capture,
                )
                .expect("failed to remove DOM event listener");
            }
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
