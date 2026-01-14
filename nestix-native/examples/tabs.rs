use std::collections::HashMap;

use env_logger::Env;
use nestix::{
    Element, callback, component, components::For, computed, create_state, layout, render_root,
};
use nestix_native::{App, Button, Input, Label, ListView, TabView, TabViewItem, Window};
use nestix_native_core::ListViewDirection;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    render_root(&layout! {ExampleApp});
}

#[component]
fn ExampleApp() -> Element {
    layout! {
        App {
            Window(
                .title = "Example App",
                .width = 300,
                .height = 300,
                .view = layout! {
                    TabView {
                        TabViewItem(
                            .id = "counter",
                            .title = "Counter",
                            .view = layout! {Counter}
                        )
                        TabViewItem(
                            .id = "todo_list",
                            .title = "Todo List",
                            .view = layout! {TodoList}
                        )
                    }
                }
            )
        }
    }
}

#[component]
fn Counter() -> Element {
    let count = create_state(0);

    layout! {
        ListView {
            Label(.text = computed!(count => || format!("Count: {}", count.get())))
            Button(
                .title = "Click",
                .on_click = callback!(count => || {
                    count.mutate(|count| *count += 1);
                })
            )
            if count.get() % 2 == 0 {
                Label(.text = "Is Even!")
            }
        }
    }
}

#[component]
fn TodoList() -> Element {
    let items = create_state::<HashMap<String, String>>(HashMap::new());
    let input_text = create_state("".to_string());

    let on_text_change = callback!(input_text => |text: &str| {
        input_text.set(text.to_string());
    });

    let add = callback!(items, input_text => || {
        let text = input_text.get();
        if !text.is_empty() {
            items.mutate(|items| {
                items.insert(nanoid::nanoid!(), text);
            });
            input_text.set("".to_string());
        }
    });

    layout! {
        ListView {
            ListView(.direction = ListViewDirection::Horizontal) {
                Input(.value = input_text, .on_text_change = on_text_change)
                Button(.title = "Add", .on_click = add)
            }
            ListView {
                For<_, HashMap<String, String>, String>(
                    .data = items,
                    .key = callback!(|(k, _): &(String, String)| k.clone()),
                    .constructor = callback!(|(_, v): &(String, String)| layout! {
                        Label(.text = v.clone())
                    })
                )
            }
        }
    }
}
