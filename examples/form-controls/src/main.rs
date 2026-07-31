use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, BackendCase, Button, Checkbox, FlexDirection, FlexView, Input, ListView,
    ListViewItem, RadioButton, Root, Select, SelectOption, Slider, StyleProvider, Switch, TabView,
    TabViewItem, TableView, TableViewCell, TableViewColumn, TableViewRow, Text, TreeView,
    TreeViewItem, Window, style,
};

#[derive(Clone, PartialEq, Eq)]
struct Person {
    id: String,
    name: String,
    role: String,
}

#[derive(Clone, PartialEq, Eq)]
struct Folder {
    id: String,
    label: String,
    children: Vec<Folder>,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { FormControlsApp });
}

#[component]
fn FormControlsApp() -> Element {
    let (name, set_name) = create_state(String::new());
    let (newsletter, set_newsletter) = create_state(false);
    let (notifications, set_notifications) = create_state(true);
    let (density, set_density) = create_state("comfortable".to_string());
    let (country, set_country) = create_state(None::<String>);
    let (volume, set_volume) = create_state(50.0);
    let (status, set_status) = create_state("Complete the form, then press Save.".to_string());

    let styles = style! {
        .content {
            padding: 28 px;
        }

        .heading {
            font_size: 24 px;
            margin_bottom: 6 px;
        }

        .description {
            margin_bottom: 22 px;
        }

        .label {
            margin_bottom: 6 px;
        }

        .field {
            margin_bottom: 16 px;
        }

        .choice {
            margin_right: 18 px;
        }

        .actions {
            margin_top: 8 px;
            margin_bottom: 18 px;
        }

        .actions > .__Button {
            margin_right: 10 px;
        }
    };

    layout! {
        StyleProvider(styles) {
            Root {
                Window(
                    .title = "Nestix Form Controls",
                    .desktop(
                        .width = 560,
                        .height = 680,
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    ),
                ) {
                    TabView(.view(.flex_grow = 1.0)) {
                        TabViewItem(.id = "form", .title = "Form") {
                            FlexView(.class = "content", .view(.flex_grow = 1.0)) {
                                Text("Form controls", .class = "heading")
                                Text(
                                    "Controlled native components exposed through nestix-native.",
                                    .class = "description",
                                )
                                Text("Name", .class = "label")
                                Input(
                                    .class = "field",
                                    .view(.width = 320),
                                    .value = name.clone(),
                                    .placeholder = "Enter your name…",
                                    .on_text_change = callback!(
                                        [set_name] |value: &str| {
                                            set_name.set(value.to_string());
                                        }
                                    ),
                                )
                                Checkbox(
                                    "Subscribe to the newsletter",
                                    .class = "field",
                                    .checked = newsletter.clone(),
                                    .on_checked_change = callback!(
                                        [set_newsletter] | checked | {
                                            set_newsletter.set(checked);
                                        }
                                    ),
                                )
                                Text("Interface density", .class = "label")
                                FlexView(
                                    .class = "field",
                                    .flex_direction = FlexDirection::Row,
                                    .align_items = AlignItems::Center,
                                ) {
                                    RadioButton(
                                        "Compact",
                                        .class = "choice",
                                        .group = "density",
                                        .selected = computed!(
                                            [density] || density.get() == "compact"
                                        ),
                                        .on_select = callback!(
                                            [set_density] || {
                                                set_density.set("compact".to_string());
                                            }
                                        ),
                                    )
                                    RadioButton(
                                        "Comfortable",
                                        .group = "density",
                                        .selected = computed!(
                                            [density] || density.get() == "comfortable"
                                        ),
                                        .on_select = callback!(
                                            [set_density] || {
                                                set_density.set("comfortable".to_string());
                                            }
                                        ),
                                    )
                                }
                                Text("Country", .class = "label")
                                Select(
                                    .class = "field",
                                    .view(.width = 220),
                                    .value = country.clone(),
                                    .on_value_change = callback!(
                                        [set_country] |value: &str| {
                                            set_country.set(Some(value.to_string()));
                                        }
                                    ),
                                ) {
                                    SelectOption("Australia", .value = "au")
                                    SelectOption("New Zealand", .value = "nz")
                                    SelectOption("United States", .value = "us")
                                    SelectOption(
                                        "Unavailable choice",
                                        .value = "disabled",
                                        .enabled = false,
                                    )
                                }
                                Text(
                                    computed!(
                                        [volume] || format!("Volume: {:.0}", volume.get())
                                    ),
                                    .class = "label",
                                )
                                Slider(
                                    .class = "field",
                                    .view(.width = 320),
                                    .value = volume.clone(),
                                    .minimum = 0.0,
                                    .maximum = 100.0,
                                    .on_value_change = callback!(
                                        [set_volume] | value | {
                                            set_volume.set(value);
                                        }
                                    ),
                                )
                                FlexView(
                                    .class = "field",
                                    .flex_direction = FlexDirection::Row,
                                    .align_items = AlignItems::Center,
                                ) {
                                    BackendCase(
                                        "nestix-native-win32",
                                        .replacement = layout! {
                                            Checkbox(
                                                "Enable notifications",
                                                .checked = notifications.clone(),
                                                .on_checked_change = callback!(
                                                    [set_notifications] | checked | {
                                                        set_notifications.set(checked);
                                                    }
                                                ),
                                            )
                                        },
                                    ) {
                                        Text("Enable notifications", .class = "choice")
                                        Switch(
                                            .checked = notifications.clone(),
                                            .on_checked_change = callback!(
                                                [set_notifications] | checked | {
                                                    set_notifications.set(checked);
                                                }
                                            ),
                                        )
                                    }
                                }
                                FlexView(
                                    .class = "actions",
                                    .flex_direction = FlexDirection::Row,
                                    .align_items = AlignItems::Center,
                                ) {
                                    Button(
                                        .title = "Save",
                                        .disabled = computed!(
                                            [name] || name.get().trim().is_empty()
                                        ),
                                        .on_click = callback!(
                                            [
                                                name,
                                                newsletter,
                                                notifications,
                                                density,
                                                country,
                                                volume,
                                                set_status,
                                            ] || {
                                                let country = country
                                                    .get()
                                                    .unwrap_or_else(|| "not selected".to_string());
                                                set_status.set(format!(
                                                    "Saved: name={:?}, newsletter={}, notifications={}, density={}, country={}, volume={:.0}",
                                                    name.get(),
                                                    newsletter.get(),
                                                    notifications.get(),
                                                    density.get(),
                                                    country,
                                                    volume.get(),
                                                ));
                                            }
                                        ),
                                    )
                                    Button(
                                        .title = "Reset",
                                        .disabled = computed!(
                                            [
                                                name,
                                                newsletter,
                                                notifications,
                                                density,
                                                country,
                                                volume
                                            ] || {
                                                name.get().is_empty()
                                                    && !newsletter.get()
                                                    && notifications.get()
                                                    && density.get() == "comfortable"
                                                    && country.get().is_none()
                                                    && volume.get() == 50.0
                                            }
                                        ),
                                        .on_click = callback!(
                                            [
                                                set_name,
                                                set_newsletter,
                                                set_notifications,
                                                set_density,
                                                set_country,
                                                set_volume,
                                                set_status
                                            ] || {
                                                set_name.set(String::new());
                                                set_newsletter.set(false);
                                                set_notifications.set(true);
                                                set_density.set("comfortable".to_string());
                                                set_country.set(None);
                                                set_volume.set(50.0);
                                                set_status.set("Form reset.".to_string());
                                            }
                                        ),
                                    )
                                }
                                Text(status)
                            }
                        }
                        TabViewItem(.id = "collections", .title = "Collections") {
                            CollectionControls
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CollectionControls() -> Element {
    let people = vec![
        Person {
            id: "ada".to_string(),
            name: "Ada Lovelace".to_string(),
            role: "Mathematician".to_string(),
        },
        Person {
            id: "grace".to_string(),
            name: "Grace Hopper".to_string(),
            role: "Computer scientist".to_string(),
        },
        Person {
            id: "margaret".to_string(),
            name: "Margaret Hamilton".to_string(),
            role: "Software engineer".to_string(),
        },
    ];
    let folders = vec![Folder {
        id: "projects".to_string(),
        label: "Projects".to_string(),
        children: vec![
            Folder {
                id: "nestix".to_string(),
                label: "nestix".to_string(),
                children: vec![],
            },
            Folder {
                id: "nestix-native".to_string(),
                label: "nestix-native".to_string(),
                children: vec![Folder {
                    id: "examples".to_string(),
                    label: "examples".to_string(),
                    children: vec![],
                }],
            },
        ],
    }];
    let (list_value, set_list_value) = create_state(None::<String>);
    let (table_value, set_table_value) = create_state(None::<String>);
    let (tree_value, set_tree_value) = create_state(None::<String>);
    let (status, set_status) = create_state("Select or activate an item.".to_string());

    layout! {
        FlexView(.class = "content", .view(.flex_grow = 1.0)) {
            Text("Collection views", .class = "heading")
            Text(
                "List, table, and tree controls using keyed data renderers.",
                .class = "description",
            )
            FlexView(.flex_direction = FlexDirection::Row, .view(.flex_grow = 1.0), .gap = 20) {
                FlexView(.view(.width = 220, .margin_right = 14)) {
                    Text("ListView", .class = "label")
                    ListView<Person>(
                        .view(.height = 180),
                        .items = people.clone(),
                        .key = callback!(|person: &Person| person.id.clone()),
                        .value = list_value.clone(),
                        .on_value_change = callback!(
                            [set_list_value] |value: &str| {
                                set_list_value.set(Some(value.to_string()));
                            }
                        ),
                        .on_activate = callback!(
                            [set_status] |value: &str| {
                                set_status.set(format!("Activated list item: {value}"));
                            }
                        ),
                    ) |person: nestix::Readonly<Person>| {
                        ListViewItem(computed!([person] || person.get().name))
                    }
                    Text("TreeView", .class = "label", .view(.margin_top = 14))
                    TreeView<Folder>(
                        .view(.height = 210),
                        .items = folders,
                        .key = callback!(|folder: &Folder| folder.id.clone()),
                        .child_items = callback!(|folder: &Folder| folder.children.clone()),
                        .value = tree_value.clone(),
                        .on_value_change = callback!(
                            [set_tree_value] |value: &str| {
                                set_tree_value.set(Some(value.to_string()));
                            }
                        ),
                        .on_activate = callback!(
                            [set_status] |value: &str| {
                                set_status.set(format!("Activated tree item: {value}"));
                            }
                        ),
                    ) |folder: nestix::Readonly<Folder>| {
                        TreeViewItem(computed!([folder] || folder.get().label))
                    }
                }
                FlexView(.view(.flex_grow = 1.0)) {
                    Text("TableView", .class = "label")
                    TableView<Person>(
                        .view(.height = 404, .flex_grow = 1.0),
                        .items = people,
                        .key = callback!(|person: &Person| person.id.clone()),
                        .columns = vec![
                            TableViewColumn::new("name", "Name"),
                            TableViewColumn::new("role", "Role"),
                        ],
                        .value = table_value.clone(),
                        .on_value_change = callback!(
                            [set_table_value] |value: &str| {
                                set_table_value.set(Some(value.to_string()));
                            }
                        ),
                        .on_activate = callback!(
                            [set_status] |value: &str| {
                                set_status.set(format!("Activated table row: {value}"));
                            }
                        ),
                    ) |person: nestix::Readonly<Person>| {
                        TableViewRow {
                            TableViewCell(
                                computed!([person] || person.get().name),
                                .column = "name",
                            )
                            TableViewCell(
                                computed!([person] || person.get().role),
                                .column = "role",
                            )
                        }
                    }
                }
            }
            Text(status.clone(), .view(.margin_top = 14))
        }
    }
}
