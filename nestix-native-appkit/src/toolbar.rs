use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nestix::{
    Element, Layout, PropValue, Shared, closure, component, components::ContextProvider, layout,
    props, scoped_effect,
};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject, sel,
};
use objc2_app_kit::{
    NSImage, NSToolbar, NSToolbarDelegate, NSToolbarDisplayMode,
    NSToolbarFlexibleSpaceItemIdentifier, NSToolbarItem, NSToolbarItemIdentifier,
    NSToolbarSpaceItemIdentifier,
};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};

use crate::WindowContext;

/// Controls whether toolbar items show their symbols, labels, or both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppKitToolbarDisplayMode {
    #[default]
    Default,
    IconAndLabel,
    IconOnly,
    LabelOnly,
}

impl AppKitToolbarDisplayMode {
    fn to_native(self) -> NSToolbarDisplayMode {
        match self {
            Self::Default => NSToolbarDisplayMode::Default,
            Self::IconAndLabel => NSToolbarDisplayMode::IconAndLabel,
            Self::IconOnly => NSToolbarDisplayMode::IconOnly,
            Self::LabelOnly => NSToolbarDisplayMode::LabelOnly,
        }
    }
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct AppKitToolbarProps {
    /// Stable identifier used by AppKit to identify this toolbar.
    pub identifier: String,

    #[props(default = true)]
    pub visible: bool,

    #[props(default)]
    pub display_mode: AppKitToolbarDisplayMode,

    #[props(default)]
    pub children: Layout,
}

#[props(debug)]
#[derive(Debug, Clone)]
pub struct AppKitToolbarItemProps {
    /// Stable identifier unique among the custom items in this toolbar.
    pub identifier: String,

    #[props(default)]
    pub label: String,

    pub symbol_name: Option<String>,
    pub accessibility_description: Option<String>,
    pub tool_tip: Option<String>,

    #[props(default)]
    pub disabled: bool,

    #[props(default)]
    pub hidden: bool,

    #[props(default)]
    pub bordered: bool,

    pub on_click: Option<Shared<dyn Fn()>>,
}

#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct AppKitToolbarSpaceProps {}

#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct AppKitToolbarFlexibleSpaceProps {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolbarRegistration {
    key: String,
    identifier: String,
}

#[derive(Clone)]
struct ToolbarContext {
    toolbar: Retained<NSToolbar>,
    registrations: Rc<RefCell<Vec<ToolbarRegistration>>>,
    items: Rc<RefCell<HashMap<String, ToolbarItemDefinition>>>,
}

impl ToolbarContext {
    fn register_item(&self, identifier: &str, item: ToolbarItemDefinition) {
        assert!(
            !identifier.is_empty(),
            "AppKitToolbarItem identifier must not be empty"
        );
        let is_reserved = unsafe {
            identifier == NSToolbarSpaceItemIdentifier.to_string()
                || identifier == NSToolbarFlexibleSpaceItemIdentifier.to_string()
        };
        assert!(
            !is_reserved,
            "AppKitToolbarItem identifier `{identifier}` is reserved by AppKit"
        );

        let mut items = self.items.borrow_mut();
        assert!(
            !items.contains_key(identifier),
            "duplicate AppKitToolbarItem identifier `{identifier}`"
        );
        items.insert(identifier.to_string(), item);
    }

    fn unregister_item(&self, identifier: &str) {
        self.items.borrow_mut().remove(identifier);
    }

    fn place(&self, key: &str, identifier: &str, index: Option<usize>) {
        place_registration(
            &mut self.registrations.borrow_mut(),
            ToolbarRegistration {
                key: key.to_string(),
                identifier: identifier.to_string(),
            },
            index,
        );
        self.sync();
    }

    fn remove(&self, key: &str) {
        self.registrations
            .borrow_mut()
            .retain(|registration| registration.key != key);
        self.sync();
    }

    fn sync(&self) {
        let current_count = self.toolbar.items().len();
        for index in (0..current_count).rev() {
            self.toolbar.removeItemAtIndex(index as _);
        }
        for definition in self.items.borrow().values() {
            definition.instances.borrow_mut().clear();
        }
        for registration in self.registrations.borrow().iter() {
            let hidden = self
                .items
                .borrow()
                .get(&registration.identifier)
                .is_some_and(|definition| definition.hidden.get());
            if hidden {
                continue;
            }
            self.toolbar.insertItemWithItemIdentifier_atIndex(
                &NSString::from_str(&registration.identifier),
                self.toolbar.items().len() as _,
            );
        }
    }
}

#[derive(Clone)]
struct ToolbarItemDefinition {
    create: Shared<dyn Fn() -> Retained<NSToolbarItem>>,
    instances: Rc<RefCell<Vec<Retained<NSToolbarItem>>>>,
    hidden: PropValue<bool>,
}

fn place_registration(
    registrations: &mut Vec<ToolbarRegistration>,
    registration: ToolbarRegistration,
    index: Option<usize>,
) {
    registrations.retain(|current| current.key != registration.key);
    let index = index
        .unwrap_or(registrations.len())
        .min(registrations.len());
    registrations.insert(index, registration);
}

/// Attaches an AppKit toolbar to the nearest containing [`crate::Window`].
#[component]
pub fn AppKitToolbar(props: &AppKitToolbarProps, element: &Element) -> Element {
    let window = element
        .context::<WindowContext>()
        .expect("AppKitToolbar must be mounted beneath an AppKit Window");
    assert!(
        window.toolbar.get().is_none(),
        "an AppKit Window can only contain one mounted AppKitToolbar"
    );

    let identifier = props.identifier.get();
    assert!(
        !identifier.is_empty(),
        "AppKitToolbar identifier must not be empty"
    );

    let mtm = MainThreadMarker::new().expect("AppKitToolbar must be mounted on the main thread");
    let toolbar =
        NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), &NSString::from_str(&identifier));
    let items = Rc::new(RefCell::new(HashMap::new()));
    let registrations = Rc::new(RefCell::new(Vec::new()));
    let delegate = AppKitToolbarDelegate::new(
        mtm,
        AppKitToolbarDelegateState {
            items: items.clone(),
            registrations: registrations.clone(),
        },
    );
    toolbar.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    let context = ToolbarContext {
        toolbar: toolbar.clone(),
        registrations,
        items,
    };

    window.toolbar.set(Some(toolbar.clone()));
    window.ns_window.setToolbar(Some(&toolbar));

    scoped_effect!(
        element,
        [toolbar, props.visible] || {
            toolbar.setVisible(visible.get());
        }
    );
    scoped_effect!(
        element,
        [toolbar, props.display_mode] || {
            toolbar.setDisplayMode(display_mode.get().to_native());
        }
    );

    element.on_unmount(closure!(
        [window, toolbar, delegate] || {
            toolbar.setDelegate(None);
            if contains_toolbar(&window.toolbar.get(), &toolbar) {
                window.toolbar.set(None);
            }
            if window.ns_window.toolbar().is_some_and(|current| {
                std::ptr::eq::<NSToolbar>(current.as_ref(), toolbar.as_ref())
            }) {
                window.ns_window.setToolbar(None);
            }
            let _ = &delegate;
        }
    ));

    layout! {
        ContextProvider<ToolbarContext>(context) {
            $(props.children.clone())
        }
    }
}

/// Adds an actionable native item to an [`AppKitToolbar`].
#[component]
pub fn AppKitToolbarItem(props: &AppKitToolbarItemProps, element: &Element) {
    let context = element
        .context::<ToolbarContext>()
        .expect("AppKitToolbarItem must be mounted beneath AppKitToolbar");
    let identifier = props.identifier.get();
    let mtm =
        MainThreadMarker::new().expect("AppKitToolbarItem must be mounted on the main thread");
    let handler = AppKitToolbarItemHandler::new(
        mtm,
        AppKitToolbarItemHandlerState {
            on_click: props.on_click.clone(),
        },
    );
    let instances = Rc::new(RefCell::new(Vec::new()));
    let create: Shared<dyn Fn() -> Retained<NSToolbarItem>> = Shared::from(Rc::new({
        let identifier = identifier.clone();
        let handler = handler.clone();
        let label = props.label.clone();
        let symbol_name = props.symbol_name.clone();
        let accessibility_description = props.accessibility_description.clone();
        let tool_tip = props.tool_tip.clone();
        let disabled = props.disabled.clone();
        let bordered = props.bordered.clone();
        move || {
            let item = NSToolbarItem::initWithItemIdentifier(
                NSToolbarItem::alloc(mtm),
                &NSString::from_str(&identifier),
            );
            unsafe {
                item.setTarget(Some(&handler));
                item.setAction(Some(sel!(activate:)));
            }
            update_item_label(&item, &label.get());
            update_item_tool_tip(&item, tool_tip.get());
            update_item_state(&item, disabled.get(), bordered.get());
            update_item_image(&item, symbol_name.get(), accessibility_description.get());
            item
        }
    })
        as Rc<dyn Fn() -> Retained<NSToolbarItem>>);

    context.register_item(
        &identifier,
        ToolbarItemDefinition {
            create,
            instances: instances.clone(),
            hidden: props.hidden.clone(),
        },
    );

    let registration_key = nanoid::nanoid!();
    element.on_place(closure!(
        [context, identifier, registration_key] | placement | {
            context.place(&registration_key, &identifier, placement.index);
        }
    ));
    element.on_unmount(closure!(
        [context, identifier, registration_key] || {
            context.remove(&registration_key);
            context.unregister_item(&identifier);
        }
    ));

    scoped_effect!(
        element,
        [instances, props.label] || {
            let label = label.get();
            for item in instances.borrow().iter() {
                update_item_label(item, &label);
            }
        }
    );
    scoped_effect!(
        element,
        [instances, props.tool_tip] || {
            let tool_tip = tool_tip.get();
            for item in instances.borrow().iter() {
                update_item_tool_tip(item, tool_tip.clone());
            }
        }
    );
    scoped_effect!(
        element,
        [instances, props.disabled, props.bordered] || {
            let disabled = disabled.get();
            let bordered = bordered.get();
            for item in instances.borrow().iter() {
                update_item_state(item, disabled, bordered);
            }
        }
    );
    scoped_effect!(
        element,
        [context, props.hidden] || {
            let _ = hidden.get();
            context.sync();
        }
    );
    scoped_effect!(
        element,
        [
            instances,
            props.symbol_name,
            props.accessibility_description
        ] || {
            let symbol_name = symbol_name.get();
            let accessibility_description = accessibility_description.get();
            for item in instances.borrow().iter() {
                update_item_image(item, symbol_name.clone(), accessibility_description.clone());
            }
        }
    );
}

fn update_item_label(item: &NSToolbarItem, label: &str) {
    let label = NSString::from_str(label);
    item.setLabel(&label);
    item.setPaletteLabel(&label);
}

fn update_item_tool_tip(item: &NSToolbarItem, tool_tip: Option<String>) {
    let tool_tip = tool_tip.map(|value| NSString::from_str(&value));
    item.setToolTip(tool_tip.as_deref());
}

fn update_item_state(item: &NSToolbarItem, disabled: bool, bordered: bool) {
    item.setEnabled(!disabled);
    item.setBordered(bordered);
}

fn update_item_image(
    item: &NSToolbarItem,
    symbol_name: Option<String>,
    accessibility_description: Option<String>,
) {
    let description = accessibility_description.map(|value| NSString::from_str(&value));
    let image = symbol_name.and_then(|name| {
        NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(&name),
            description.as_deref(),
        )
    });
    item.setImage(image.as_deref());
}

/// Adds a fixed-width native space to an [`AppKitToolbar`].
#[component]
pub fn AppKitToolbarSpace(_: &AppKitToolbarSpaceProps, element: &Element) {
    mount_space(element, unsafe { NSToolbarSpaceItemIdentifier });
}

/// Adds a flexible native space to an [`AppKitToolbar`].
#[component]
pub fn AppKitToolbarFlexibleSpace(_: &AppKitToolbarFlexibleSpaceProps, element: &Element) {
    mount_space(element, unsafe { NSToolbarFlexibleSpaceItemIdentifier });
}

fn mount_space(element: &Element, identifier: &NSToolbarItemIdentifier) {
    let context = element
        .context::<ToolbarContext>()
        .expect("toolbar spaces must be mounted beneath AppKitToolbar");
    let identifier = identifier.to_string();
    let registration_key = nanoid::nanoid!();
    element.on_place(closure!(
        [context, identifier, registration_key] | placement | {
            context.place(&registration_key, &identifier, placement.index);
        }
    ));
    element.on_unmount(closure!(
        [context, registration_key] || {
            context.remove(&registration_key);
        }
    ));
}

fn contains_toolbar(slot: &Option<Retained<NSToolbar>>, toolbar: &NSToolbar) -> bool {
    slot.as_ref()
        .is_some_and(|current| std::ptr::eq(current.as_ref(), toolbar))
}

struct AppKitToolbarDelegateState {
    items: Rc<RefCell<HashMap<String, ToolbarItemDefinition>>>,
    registrations: Rc<RefCell<Vec<ToolbarRegistration>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixAppKitToolbarDelegate"]
    #[ivars = AppKitToolbarDelegateState]
    struct AppKitToolbarDelegate;

    unsafe impl NSObjectProtocol for AppKitToolbarDelegate {}

    unsafe impl NSToolbarDelegate for AppKitToolbarDelegate {
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn toolbar_item_for_item_identifier_will_be_inserted_into_toolbar(
            &self,
            _: &NSToolbar,
            identifier: &NSToolbarItemIdentifier,
            _: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            let definition = self
                .ivars()
                .items
                .borrow()
                .get(&identifier.to_string())
                .cloned();
            if let Some(definition) = definition {
                let item = (definition.create)();
                definition.instances.borrow_mut().push(item.clone());
                Some(item)
            } else {
                None
            }
        }

        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn toolbar_default_item_identifiers(
            &self,
            _: &NSToolbar,
        ) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            toolbar_identifiers(&self.ivars().registrations.borrow())
        }

        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn toolbar_allowed_item_identifiers(
            &self,
            _: &NSToolbar,
        ) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            toolbar_identifiers(&self.ivars().registrations.borrow())
        }
    }
);

fn toolbar_identifiers(
    registrations: &[ToolbarRegistration],
) -> Retained<NSArray<NSToolbarItemIdentifier>> {
    let identifiers = registrations
        .iter()
        .map(|registration| NSString::from_str(&registration.identifier))
        .collect::<Vec<_>>();
    NSArray::from_retained_slice(&identifiers)
}

impl AppKitToolbarDelegate {
    fn new(mtm: MainThreadMarker, state: AppKitToolbarDelegateState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Debug)]
struct AppKitToolbarItemHandlerState {
    on_click: PropValue<Option<Shared<dyn Fn()>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixAppKitToolbarItemHandler"]
    #[ivars = AppKitToolbarItemHandlerState]
    struct AppKitToolbarItemHandler;

    unsafe impl NSObjectProtocol for AppKitToolbarItemHandler {}

    impl AppKitToolbarItemHandler {
        #[unsafe(method(activate:))]
        fn activate(&self, _: &NSToolbarItem) {
            if let Some(on_click) = self.ivars().on_click.get() {
                on_click();
            }
        }
    }
);

impl AppKitToolbarItemHandler {
    fn new(mtm: MainThreadMarker, state: AppKitToolbarItemHandlerState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolbarRegistration, place_registration};

    fn registration(key: &str, identifier: &str) -> ToolbarRegistration {
        ToolbarRegistration {
            key: key.to_string(),
            identifier: identifier.to_string(),
        }
    }

    #[test]
    fn placement_inserts_and_moves_registrations() {
        let mut registrations = Vec::new();
        place_registration(&mut registrations, registration("one", "first"), None);
        place_registration(&mut registrations, registration("two", "second"), None);
        place_registration(&mut registrations, registration("three", "third"), Some(1));
        assert_eq!(
            registrations
                .iter()
                .map(|registration| registration.identifier.as_str())
                .collect::<Vec<_>>(),
            ["first", "third", "second"]
        );

        place_registration(&mut registrations, registration("one", "first"), Some(2));
        assert_eq!(
            registrations
                .iter()
                .map(|registration| registration.identifier.as_str())
                .collect::<Vec<_>>(),
            ["third", "second", "first"]
        );
    }

    #[test]
    fn placement_allows_repeated_space_identifiers() {
        let mut registrations = Vec::new();
        place_registration(&mut registrations, registration("one", "space"), None);
        place_registration(&mut registrations, registration("two", "space"), None);
        assert_eq!(registrations.len(), 2);
        assert_eq!(registrations[0].identifier, registrations[1].identifier);
    }
}
