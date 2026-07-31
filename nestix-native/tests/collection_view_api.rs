#![allow(dead_code, non_snake_case)]

use nestix::{Element, Readonly, callback, component, computed, create_state, layout};
use nestix_native::{
    FlexView, ListView, ListViewItem, TableView, TableViewCell, TableViewColumn, TableViewRow,
    TreeView, TreeViewItem,
};

#[derive(Clone, PartialEq, Eq)]
struct Item {
    id: String,
    label: String,
    detail: String,
    children: Vec<Item>,
}

#[component]
fn CollectionViews() -> Element {
    let (items, _) = create_state(vec![Item {
        id: "root".to_string(),
        label: "Root".to_string(),
        detail: "Detail".to_string(),
        children: vec![],
    }]);
    let (value, set_value) = create_state(None::<String>);

    layout! {
        FlexView {
            ListView<Item>(
                .items = items.clone(),
                .key = callback!(|item: &Item| item.id.clone()),
                .value = value.clone(),
                .on_value_change = callback!(
                    [set_value] |value: &str| {
                        set_value.set(Some(value.to_string()));
                    }
                ),
                .on_activate = callback!(|_value: &str| {}),
            ) |item: Readonly<Item>| {
                ListViewItem(computed!([item] || item.get().label))
            }
            TableView<Item>(
                .items = items.clone(),
                .key = callback!(|item: &Item| item.id.clone()),
                .columns = vec![
                    TableViewColumn::new("label", "Label"),
                    TableViewColumn::new("detail", "Detail"),
                ],
                .value = value.clone(),
            ) |item: Readonly<Item>| {
                TableViewRow {
                    TableViewCell(computed!([item] || item.get().label), .column = "label")
                    TableViewCell(computed!([item] || item.get().detail), .column = "detail")
                }
            }
            TreeView<Item>(
                .items = items,
                .key = callback!(|item: &Item| item.id.clone()),
                .child_items = callback!(|item: &Item| item.children.clone()),
                .value = value,
            ) |item: Readonly<Item>| {
                TreeViewItem(computed!([item] || item.get().label))
            }
        }
    }
}

#[test]
fn collection_views_compile_through_layout() {
    let _views = layout! { CollectionViews };
}
