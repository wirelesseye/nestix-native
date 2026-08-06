use nestix::{
    Element, ElementId, PropValue, Readonly, Shared, build_props, callback, component, computed,
    create_element, create_state, layout, props, scoped_effect,
};

use crate::{
    Checkbox, FlexDirection, FlexView, ScrollView, Text, TreeView, TreeViewItem, TreeViewItemProps,
    Window,
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
    props: String,
    children: Vec<Self>,
}

impl InspectorNode {
    fn key(&self) -> String {
        self.id.get().to_string()
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
    let details = computed!(
        [nodes, selection] || {
            let nodes = nodes.get();
            let selected = selection.get();
            selected
                .as_deref()
                .and_then(|key| find_node(&nodes, key))
                .map(format_details)
                .unwrap_or_else(|| "Select a component".to_string())
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
                    ScrollView(.view(.width = 400)) {
                        FlexView {
                            Text(details.clone(), .view(.flex_grow = 1.0))
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
        props: format!("{:#?}", element.props()),
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

fn format_details(node: &InspectorNode) -> String {
    format!(
        "Component: {}\nType: {}\nElement ID: {}\nVisibility: {}\n\nProps\n{}",
        node.label,
        node.full_name,
        node.id.get(),
        if node.internal { "Internal" } else { "Public" },
        node.props,
    )
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
        assert!(projected[0].children[0].props.contains("visible"));

        let detailed = snapshot_children(&root, excluded.id(), true);
        assert_eq!(detailed[0].children.len(), 1);
        assert_eq!(detailed[0].children[0].id, internal.id());
        assert_eq!(detailed[0].children[0].children[0].id, visible.id());

        root.unmount();
    }

    #[test]
    fn short_name_keeps_generic_arguments_after_the_component_name() {
        assert_eq!(
            short_component_name("my_app::components::Panel<my_app::Row>"),
            "Panel<my_app::Row>"
        );
    }
}
