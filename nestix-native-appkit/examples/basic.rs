use std::collections::HashMap;

use env_logger::Env;
use nestix::{
    Element, callback, component, components::For, computed, create_state, layout, render,
};
use nestix_native_appkit::{
    AppkitInput, AppkitTabView, AppkitTabViewItem, app::AppkitApp, button::AppkitButton,
    label::AppkitLabel, list_view::AppkitListView, window::AppkitWindow,
};
use nestix_native_core::ListViewDirection;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    render(&layout! {BasicApp});
}

#[component]
fn BasicApp() -> Element {
    layout! {
        AppkitApp {
            AppkitWindow(
                .title = "Counter",
                .width = 300,
                .height = 300,
                .view = layout! {
                    AppkitTabView {
                        AppkitTabViewItem(
                            .id = "counter",
                            .title = "Counter",
                            .view = layout! {Counter}
                        )
                        AppkitTabViewItem(
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
        AppkitListView {
            AppkitLabel(.text = computed!(count => || format!("Count: {}", count.get())))
            AppkitButton(
                .title = "Click",
                .on_click = callback!(count => || {
                    count.mutate(|count| *count += 1);
                })
            )
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
        AppkitListView {
            AppkitListView(.direction = ListViewDirection::Horizontal) {
                AppkitInput(.value = input_text, .on_text_change = on_text_change)
                AppkitButton(.title = "Add", .on_click = add)
            }
            AppkitListView {
                For<_, HashMap<String, String>, String>(
                    .data = items,
                    .key = callback!(|(k, _): &(String, String)| k.clone()),
                    .constructor = callback!(|(_, v): &(String, String)| layout! {
                        AppkitLabel(.text = v.clone())
                    })
                )
            }
        }
    }
}
