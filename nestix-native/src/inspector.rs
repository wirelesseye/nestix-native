use nestix::{
    Element, ElementId, InspectProp, InspectPropSource, PropValue, Readonly, Shared, WeakElement,
    build_props, callback, component, computed, create_element, create_state, layout, props,
    scoped_effect,
};

use crate::{
    Checkbox, FlexDirection, FlexView, TableView, TableViewCell, TableViewColumn, TableViewRow,
    Text, TreeView, TreeViewItem, TreeViewItemProps, Window,
};

/// Properties for [`LayoutInspector`].
#[props(debug)]
#[derive(Debug, Clone)]
pub struct LayoutInspectorProps {
    /// Whether the inspector window is visible.
    #[props(default = true)]
    pub visible: bool,

    /// Called when the user asks to close the inspector window.
    pub on_close_requested: Option<Shared<dyn Fn()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectorNode {
    id: ElementId,
    label: String,
    full_name: &'static str,
    internal: bool,
    element: WeakElement,
    children: Vec<Self>,
}

impl InspectorNode {
    fn key(&self) -> String {
        self.id.get().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectorPropRow {
    name: String,
    value: String,
    source: String,
    type_name: String,
}

impl InspectorPropRow {
    fn key(&self) -> String {
        self.name.clone()
    }
}

/// Displays the containing Nestix component tree in an independent window.
#[component(internal)]
pub fn LayoutInspector(props: &LayoutInspectorProps, element: &Element) -> Element {
    let root = tree_root(element);
    let root = root.downgrade();
    let excluded_subtree = element.id();
    let (revision, set_revision) = create_state(0_u64);
    let observer_set_revision = set_revision.clone();
    let observer = root
        .upgrade()
        .expect("LayoutInspector root must remain mounted while it mounts")
        .observe_tree([excluded_subtree], move || {
            observer_set_revision.update(|revision| revision.wrapping_add(1));
        });
    element.on_unmount(move || observer.cancel());

    let (show_internal, set_show_internal) = create_state(false);
    let (selection, set_selection) = create_state(None::<String>);
    let nodes = computed!(
        [revision, show_internal, root] || {
            let _ = revision.get();
            root.upgrade()
                .map(|root| snapshot_children(&root, excluded_subtree, show_internal.get()))
                .unwrap_or_default()
        }
    );
    let selected_node = computed!(
        [nodes, selection] || {
            let nodes = nodes.get();
            let selected = selection.get();
            selected
                .as_deref()
                .and_then(|key| find_node(&nodes, key))
                .cloned()
        }
    );
    let details = computed!(
        [selected_node] || {
            selected_node
                .get()
                .as_ref()
                .map(format_details_header)
                .unwrap_or_else(|| "Select a component".to_string())
        }
    );
    let prop_rows = computed!(
        [selected_node] || {
            selected_node
                .get()
                .as_ref()
                .map(inspect_prop_rows)
                .unwrap_or_default()
        }
    );
    scoped_effect!(
        [nodes, selection, set_selection] || {
            let selection = selection.get();
            if selection
                .as_deref()
                .is_some_and(|key| find_node(&nodes.get(), key).is_none())
            {
                set_selection.set(None);
            }
        }
    );

    layout! {
        Window(
            .title = "Nestix Layout Inspector",
            .visible = props.visible.clone(),
            .desktop(
                .width = 900,
                .height = 600,
                .on_close_requested = props.on_close_requested.clone(),
            ),
        ) {
            FlexView(.view(.flex_grow = 1.0)) {
                Checkbox(
                    "Show internal components",
                    .checked = show_internal,
                    .on_checked_change = callback!(
                        [set_show_internal] |checked: bool| {
                            set_show_internal.set(checked);
                        }
                    ),
                )
                FlexView(.view(.flex_grow = 1.0), .flex_direction = FlexDirection::Row) {
                    TreeView<InspectorNode>(
                        .view(.width = 360, .flex_grow = 1.0),
                        .items = nodes,
                        .key = callback!(|node: &InspectorNode| node.key()),
                        .child_items = callback!(
                            |node: &InspectorNode| node.children.clone()
                        ),
                        .value = selection,
                        .on_value_change = callback!(
                            [set_selection] |value: &str| {
                                set_selection.set(Some(value.to_string()));
                            }
                        ),
                        .children = callback!(|item: Readonly<InspectorNode>| {
                            let label = computed!([item] || item.get().label);
                            PropValue::from_plain(create_element::<TreeViewItem>(build_props!(
                                TreeViewItemProps(label)
                            )))
                        }),
                    )
                    FlexView(.view(.width = 500, .flex_grow = 1.0)) {
                        Text(details)
                        TableView<InspectorPropRow>(
                            .view(.flex_grow = 1.0),
                            .items = prop_rows,
                            .key = callback!(|row: &InspectorPropRow| row.key()),
                            .columns = vec![
                                TableViewColumn::new("name", "Property"),
                                TableViewColumn::new("value", "Value"),
                                TableViewColumn::new("source", "Source"),
                                TableViewColumn::new("type", "Type"),
                            ],
                        ) |row: Readonly<InspectorPropRow>| {
                            TableViewRow {
                                TableViewCell(computed!([row] || row.get().name), .column = "name")
                                TableViewCell(
                                    computed!([row] || row.get().value),
                                    .column = "value",
                                )
                                TableViewCell(
                                    computed!([row] || row.get().source),
                                    .column = "source",
                                )
                                TableViewCell(
                                    computed!([row] || row.get().type_name),
                                    .column = "type",
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

fn tree_root(element: &Element) -> Element {
    let mut root = element.clone();
    while let Some(parent) = root.parent() {
        root = parent;
    }
    root
}

fn snapshot_children(
    root: &Element,
    excluded: ElementId,
    show_internal: bool,
) -> Vec<InspectorNode> {
    project_element(root, excluded, show_internal)
}

fn project_element(
    element: &Element,
    excluded: ElementId,
    show_internal: bool,
) -> Vec<InspectorNode> {
    if element.id() == excluded {
        return Vec::new();
    }

    let children = element
        .children()
        .into_iter()
        .flat_map(|child| project_element(&child, excluded, show_internal))
        .collect::<Vec<_>>();
    let internal = element.is_internal();
    if internal && !show_internal {
        return children;
    }

    let full_name = element.component_id().name();
    vec![InspectorNode {
        id: element.id(),
        label: short_component_name(full_name).to_string(),
        full_name,
        internal,
        element: element.downgrade(),
        children,
    }]
}

fn short_component_name(name: &str) -> &str {
    let base_end = name.find('<').unwrap_or(name.len());
    let base_start = name[..base_end]
        .rfind("::")
        .map(|index| index + 2)
        .unwrap_or(0);
    &name[base_start..]
}

fn find_node<'a>(nodes: &'a [InspectorNode], key: &str) -> Option<&'a InspectorNode> {
    nodes.iter().find_map(|node| {
        if node.key() == key {
            Some(node)
        } else {
            find_node(&node.children, key)
        }
    })
}

fn format_details_header(node: &InspectorNode) -> String {
    format!(
        "Component: {}\nType: {}\nElement ID: {}\nVisibility: {}",
        node.label,
        node.full_name,
        node.id.get(),
        if node.internal { "Internal" } else { "Public" },
    )
}

fn inspect_prop_rows(node: &InspectorNode) -> Vec<InspectorPropRow> {
    let Some(element) = node.element.upgrade() else {
        return vec![InspectorPropRow {
            name: "Props".to_string(),
            value: "<component unmounted>".to_string(),
            source: String::new(),
            type_name: String::new(),
        }];
    };

    if let Some(props) = element.props().as_inspectable() {
        let entries = props.inspect_props();
        let mut rows = Vec::new();
        append_prop_rows(&mut rows, &entries, "");
        rows
    } else {
        vec![InspectorPropRow {
            name: "Props".to_string(),
            value: format!("{:#?}", element.props()),
            source: "Debug".to_string(),
            type_name: String::new(),
        }]
    }
}

fn append_prop_rows(output: &mut Vec<InspectorPropRow>, props: &[InspectProp], prefix: &str) {
    for prop in props {
        let name = if prefix.is_empty() {
            prop.name.to_string()
        } else {
            format!("{prefix}.{}", prop.name)
        };

        if prop.source == InspectPropSource::Nested {
            append_prop_rows(output, &prop.children, &name);
            continue;
        }

        output.push(InspectorPropRow {
            name,
            value: prop.value.summary(),
            source: match prop.source {
                InspectPropSource::Plain => "Plain",
                InspectPropSource::Reactive => "Reactive",
                InspectPropSource::Raw => "Raw",
                InspectPropSource::Nested => unreachable!(),
            }
            .to_string(),
            type_name: prop.type_name.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use nestix::{Component, ComponentOutput, Props, create_element};

    use super::*;

    struct Public;

    impl Component for Public {
        type Props = ();

        fn on_mount(_: &Element) {}
    }

    struct Internal;

    impl Component for Internal {
        type Props = ();
        const IS_INTERNAL: bool = true;

        fn on_mount(_: &Element) {}
    }

    #[derive(Debug)]
    struct DebugProps(&'static str);

    impl Props for DebugProps {
        fn debug_fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(self, f)
        }
    }

    struct DebugComponent;

    impl Component for DebugComponent {
        type Props = DebugProps;

        fn on_mount(_: &Element) {}
    }

    #[props]
    struct StructuredProps {
        title: String,
        enabled: bool,
        class: crate::ClassList,
        #[props(nested, default)]
        view: crate::ViewProps,
    }

    struct StructuredComponent;

    impl Component for StructuredComponent {
        type Props = StructuredProps;

        fn on_mount(_: &Element) {}
    }

    #[test]
    fn projection_promotes_hidden_internal_children_and_excludes_subtrees() {
        let root = create_element::<Public>(());
        root.mount(None);
        let internal = create_element::<Internal>(());
        internal.mount(Some(&root));
        let visible_props = DebugProps("visible");
        assert_eq!(visible_props.0, "visible");
        let visible = create_element::<DebugComponent>(visible_props);
        visible.mount(Some(&internal));
        let excluded = create_element::<Public>(());
        excluded.mount(Some(&root));

        let projected = snapshot_children(&root, excluded.id(), false);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].id, root.id());
        assert_eq!(projected[0].children.len(), 1);
        assert_eq!(projected[0].children[0].id, visible.id());
        assert!(
            inspect_prop_rows(&projected[0].children[0])[0]
                .value
                .contains("visible")
        );

        let detailed = snapshot_children(&root, excluded.id(), true);
        assert_eq!(detailed[0].children.len(), 1);
        assert_eq!(detailed[0].children[0].id, internal.id());
        assert_eq!(detailed[0].children[0].children[0].id, visible.id());

        root.unmount();
    }

    #[test]
    fn details_format_generated_props_as_named_values() {
        let element = create_element::<StructuredComponent>(build_props!(StructuredProps(
            .title = "Inspector".to_string(),
            .enabled = true,
            .class = "secondary primary",
            .view(.flex_grow = 2.0),
        )));
        element.mount(None);

        let excluded = create_element::<Public>(());
        let projected = snapshot_children(&element, excluded.id(), false);
        let rows = inspect_prop_rows(&projected[0]);
        assert_eq!(rows[0].name, "title");
        assert_eq!(rows[0].value, "\"Inspector\"");
        assert_eq!(rows[0].source, "Plain");
        assert_eq!(rows[1].name, "enabled");
        assert_eq!(rows[1].value, "true");
        let class = rows.iter().find(|row| row.name == "class").unwrap();
        assert_eq!(class.value, "primary secondary");
        let position = rows.iter().find(|row| row.name == "view.position").unwrap();
        assert_eq!(position.value, "Relative");
        let flex_grow = rows
            .iter()
            .find(|row| row.name == "view.flex_grow")
            .unwrap();
        assert_eq!(flex_grow.value, "2");
        assert!(!rows[0].value.contains("PropValue"));

        element.unmount();
    }

    #[test]
    fn short_name_keeps_generic_arguments_after_the_component_name() {
        assert_eq!(
            short_component_name("my_app::components::Panel<my_app::Row>"),
            "Panel<my_app::Row>"
        );
    }
}
