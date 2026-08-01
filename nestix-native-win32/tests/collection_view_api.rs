#![allow(dead_code, non_snake_case)]

use nestix::{Element, Readonly, callback, component, computed, layout};
use nestix_native_win32::{
    FlexView, ListView, ListViewItem, TableView, TableViewCell, TableViewColumn, TableViewRow,
    TreeView, TreeViewItem,
};

#[derive(Clone, PartialEq, Eq)]
struct Item {
    id: String,
    label: String,
    children: Vec<Item>,
}

#[component]
fn DirectWin32Collections() -> Element {
    let items = vec![Item {
        id: "root".to_string(),
        label: "Root".to_string(),
        children: vec![],
    }];

    layout! {
        FlexView {
            ListView<Item>(.items = items.clone(), .key = callback!(|item: &Item| item.id.clone())) |item: Readonly<Item>| {
                ListViewItem(computed!([item] || item.get().label))
            }
            TableView<Item>(
                .items = items.clone(),
                .key = callback!(|item: &Item| item.id.clone()),
                .columns = vec![TableViewColumn::new("label", "Label")],
            ) |item: Readonly<Item>| {
                TableViewRow {
                    TableViewCell(computed!([item] || item.get().label), .column = "label")
                }
            }
            TreeView<Item>(
                .items = items,
                .key = callback!(|item: &Item| item.id.clone()),
                .child_items = callback!(|item: &Item| item.children.clone()),
            ) |item: Readonly<Item>| {
                TreeViewItem(computed!([item] || item.get().label))
            }
        }
    }
}

#[test]
fn collection_views_are_available_without_the_facade_crate() {
    let _views = layout! { DirectWin32Collections };
}
