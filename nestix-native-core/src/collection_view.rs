use std::marker::PhantomData;

use nestix::{
    Element, Layout, PropValue, Readonly, Shared, component, components::create_for_from_signal,
    computed, create_element, props,
};

use crate::{ClassList, ViewProps, create_backend_element};

/// Describes a text column displayed by [`TableViewProps`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableViewColumn {
    /// Stable identifier used by cells to select this column.
    pub id: String,
    /// User-visible column heading.
    pub title: String,
}

impl TableViewColumn {
    /// Creates a column with a stable identifier and heading.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
        }
    }
}

/// Properties for a generic list view.
#[props(bounds(T: Clone + Eq + 'static))]
#[derive(Clone)]
pub struct ListViewProps<T: Clone + Eq + 'static> {
    /// Style classes applied to the native view.
    #[props(default)]
    pub class: ClassList,
    /// Common view layout properties.
    #[props(nested, default)]
    pub view: ViewProps,
    /// Whether the view accepts user interaction.
    #[props(default = true)]
    pub enabled: bool,
    /// Reactive application data rendered by the view.
    pub items: Vec<T>,
    /// Returns the globally unique selection value for an item.
    pub key: Shared<dyn Fn(&T) -> String>,
    /// Currently selected item key.
    #[props(default)]
    pub value: Option<String>,
    /// Called when the user selects an enabled item.
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    /// Called when the user activates an enabled item.
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    /// Creates the text descriptor for an item.
    pub children: Shared<dyn Fn(Readonly<T>) -> nestix::PropValue<Element>>,
}

/// Properties for a generic table view.
#[props(bounds(T: Clone + Eq + 'static))]
#[derive(Clone)]
pub struct TableViewProps<T: Clone + Eq + 'static> {
    #[props(default)]
    pub class: ClassList,
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(default = true)]
    pub enabled: bool,
    pub items: Vec<T>,
    pub key: Shared<dyn Fn(&T) -> String>,
    /// Columns displayed by the table. Column identifiers must be unique.
    pub columns: Vec<TableViewColumn>,
    #[props(default)]
    pub value: Option<String>,
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    /// Creates one [`TableViewRowProps`] descriptor for an item.
    pub children: Shared<dyn Fn(Readonly<T>) -> nestix::PropValue<Element>>,
}

/// Properties for a generic tree view.
#[props(bounds(T: Clone + Eq + 'static))]
#[derive(Clone)]
pub struct TreeViewProps<T: Clone + Eq + 'static> {
    #[props(default)]
    pub class: ClassList,
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(default = true)]
    pub enabled: bool,
    /// Root items displayed by the tree.
    pub items: Vec<T>,
    /// Returns the globally unique selection value for an item.
    pub key: Shared<dyn Fn(&T) -> String>,
    /// Returns the direct children of an item.
    pub child_items: Shared<dyn Fn(&T) -> Vec<T>>,
    #[props(default)]
    pub value: Option<String>,
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    /// Creates one [`TreeViewItemProps`] descriptor for an item.
    pub children: Shared<dyn Fn(Readonly<T>) -> nestix::PropValue<Element>>,
}

/// Backend-facing properties for a flat list host.
#[doc(hidden)]
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ListViewHostProps {
    #[props(default)]
    pub class: ClassList,
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default)]
    pub value: Option<String>,
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    #[props(default)]
    pub children: Layout,
}

/// Backend-facing properties for a table host.
#[doc(hidden)]
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TableViewHostProps {
    #[props(default)]
    pub class: ClassList,
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(default = true)]
    pub enabled: bool,
    pub columns: Vec<TableViewColumn>,
    #[props(default)]
    pub value: Option<String>,
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    #[props(default)]
    pub children: Layout,
}

/// Backend-facing properties for a tree host.
#[doc(hidden)]
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TreeViewHostProps {
    #[props(default)]
    pub class: ClassList,
    #[props(nested, default)]
    pub view: ViewProps,
    #[props(default = true)]
    pub enabled: bool,
    #[props(default)]
    pub value: Option<String>,
    pub on_value_change: Option<Shared<dyn Fn(&str)>>,
    pub on_activate: Option<Shared<dyn Fn(&str)>>,
    #[props(default)]
    pub children: Layout,
}

/// Internal keyed item/node wrapper used by concrete backends.
#[doc(hidden)]
#[props(debug)]
#[derive(Debug, Clone)]
pub struct CollectionNodeProps {
    pub value: String,
    #[props(default)]
    pub children: Layout,
}

/// Text displayed for a list item.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct ListViewItemProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
}

/// Row metadata and text cells rendered by a table factory.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TableViewRowProps {
    #[props(default = true)]
    pub enabled: bool,
    #[props(default)]
    pub children: Layout,
}

/// Text displayed in one table column.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TableViewCellProps {
    #[props(start)]
    pub text: String,
    /// Identifier of the column that receives this cell.
    pub column: String,
}

/// Text displayed for a tree item.
#[props(debug)]
#[derive(Debug, Clone)]
pub struct TreeViewItemProps {
    #[props(start)]
    pub label: String,
    #[props(default = true)]
    pub enabled: bool,
}

/// Displays keyed application data as a native flat list.
#[component(generics(T))]
pub fn ListView<T: Clone + Eq + 'static>(
    props: &ListViewProps<T>,
    element: &Element,
) -> Option<Element> {
    let reconcile_key = props.key.clone();
    let node_key = props.key.clone();
    let renderer = props.children.clone();
    let items = computed!([props.items] || items.get());
    let children = create_for_from_signal(
        items,
        move |item| reconcile_key.get()(item),
        move |item| {
            let node_key = node_key.clone();
            let value = PropValue::from_signal(item.clone()).map(move |item| node_key.get()(item));
            let descriptor = renderer.get()(item).get();
            PropValue::from_plain(create_element::<ListViewNode>(CollectionNodeProps {
                value,
                children: PropValue::from_plain(Layout::from(descriptor)),
            }))
        },
    );
    let host_props = ListViewHostProps {
        class: props.class.clone(),
        view: props.view.clone(),
        enabled: props.enabled.clone(),
        value: props.value.clone(),
        on_value_change: props.on_value_change.clone(),
        on_activate: props.on_activate.clone(),
        children: PropValue::from_plain(Layout::from(children)),
    };
    create_backend_element(element, "ListView", |backend| {
        backend.create_list_view(host_props.clone())
    })
}

#[component]
fn ListViewNode(props: &CollectionNodeProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "ListViewItem", |backend| {
        backend.create_list_view_node(props.clone())
    })
}

/// Supplies the text and enabled state for an item rendered by [`ListView`].
#[component]
pub fn ListViewItem(props: &ListViewItemProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "ListViewItem", |backend| {
        backend.create_list_view_item(props.clone())
    })
}

/// Displays keyed application data in native text columns.
#[component(generics(T))]
pub fn TableView<T: Clone + Eq + 'static>(
    props: &TableViewProps<T>,
    element: &Element,
) -> Option<Element> {
    let reconcile_key = props.key.clone();
    let node_key = props.key.clone();
    let renderer = props.children.clone();
    let items = computed!([props.items] || items.get());
    let children = create_for_from_signal(
        items,
        move |item| reconcile_key.get()(item),
        move |item| {
            let node_key = node_key.clone();
            let value = PropValue::from_signal(item.clone()).map(move |item| node_key.get()(item));
            let descriptor = renderer.get()(item).get();
            PropValue::from_plain(create_element::<TableViewNode>(CollectionNodeProps {
                value,
                children: PropValue::from_plain(Layout::from(descriptor)),
            }))
        },
    );
    let host_props = TableViewHostProps {
        class: props.class.clone(),
        view: props.view.clone(),
        enabled: props.enabled.clone(),
        columns: props.columns.clone(),
        value: props.value.clone(),
        on_value_change: props.on_value_change.clone(),
        on_activate: props.on_activate.clone(),
        children: PropValue::from_plain(Layout::from(children)),
    };
    create_backend_element(element, "TableView", |backend| {
        backend.create_table_view(host_props.clone())
    })
}

#[component]
fn TableViewNode(props: &CollectionNodeProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "TableViewRow", |backend| {
        backend.create_table_view_node(props.clone())
    })
}

/// Supplies row metadata and cells for an item rendered by [`TableView`].
#[component]
pub fn TableViewRow(props: &TableViewRowProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "TableViewRow", |backend| {
        backend.create_table_view_row(props.clone())
    })
}

/// Supplies text for one column of a [`TableViewRow`].
#[component]
pub fn TableViewCell(props: &TableViewCellProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "TableViewCell", |backend| {
        backend.create_table_view_cell(props.clone())
    })
}

#[props(bounds(T: Clone + Eq + 'static))]
struct TreeItemsProps<T: Clone + Eq + 'static> {
    items: Vec<T>,
    key: Shared<dyn Fn(&T) -> String>,
    child_items: Shared<dyn Fn(&T) -> Vec<T>>,
    renderer: Shared<dyn Fn(Readonly<T>) -> PropValue<Element>>,
}

#[component(generics(T))]
fn TreeItems<T: Clone + Eq + 'static>(props: &TreeItemsProps<T>) -> Element {
    let reconcile_key = props.key.clone();
    let node_key = props.key.clone();
    let child_items = props.child_items.clone();
    let renderer = props.renderer.clone();
    let items = computed!([props.items] || items.get());
    create_for_from_signal(
        items,
        move |item| reconcile_key.get()(item),
        move |item| {
            let node_key = node_key.clone();
            let value_key = node_key.clone();
            let value = PropValue::from_signal(item.clone()).map(move |item| value_key.get()(item));
            let descriptor = renderer.get()(item.clone()).get();
            let child_items_for_signal = child_items.clone();
            let descendants = PropValue::from_signal(item.clone())
                .map(move |item| child_items_for_signal.get()(item));
            let descendants = create_element::<TreeItems<T>>(TreeItemsProps {
                items: descendants,
                key: node_key,
                child_items: child_items.clone(),
                renderer: renderer.clone(),
            });
            PropValue::from_plain(create_element::<TreeViewNode>(CollectionNodeProps {
                value,
                children: PropValue::from_plain(Layout::from(vec![descriptor, descendants])),
            }))
        },
    )
}

/// Displays keyed hierarchical application data as a native tree.
#[component(generics(T))]
pub fn TreeView<T: Clone + Eq + 'static>(
    props: &TreeViewProps<T>,
    element: &Element,
) -> Option<Element> {
    let children = create_element::<TreeItems<T>>(TreeItemsProps {
        items: props.items.clone(),
        key: props.key.clone(),
        child_items: props.child_items.clone(),
        renderer: props.children.clone(),
    });
    let host_props = TreeViewHostProps {
        class: props.class.clone(),
        view: props.view.clone(),
        enabled: props.enabled.clone(),
        value: props.value.clone(),
        on_value_change: props.on_value_change.clone(),
        on_activate: props.on_activate.clone(),
        children: PropValue::from_plain(Layout::from(children)),
    };
    create_backend_element(element, "TreeView", |backend| {
        backend.create_tree_view(host_props.clone())
    })
}

#[component]
fn TreeViewNode(props: &CollectionNodeProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "TreeViewItem", |backend| {
        backend.create_tree_view_node(props.clone())
    })
}

/// Supplies the text and enabled state for a node rendered by [`TreeView`].
#[component]
pub fn TreeViewItem(props: &TreeViewItemProps, element: &Element) -> Option<Element> {
    create_backend_element(element, "TreeViewItem", |backend| {
        backend.create_tree_view_item(props.clone())
    })
}
