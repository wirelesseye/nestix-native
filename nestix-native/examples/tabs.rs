use env_logger::Env;
use nestix::{
    Element, Shared, callback, component, computed, create_state, layout, mount_root, props,
};
use nestix_native::{
    Alignment, Button, Direction, FlexView, Input, Root, TabView, TabViewItem, Text, Window,
    view_props_builder::ViewPropsBuilderExtGrow,
};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {ExampleApp});
}

#[component]
fn ExampleApp() -> Element {
    layout! {
        Root {
            Window(
                .title = "Example App",
                .width = 300,
                .height = 300,
            ) {
                TabView {
                    TabViewItem(
                        .id = "counter",
                        .title = "Counter",
                    ) {
                        Counter
                    }
                    TabViewItem(
                        .id = "todo_list",
                        .title = "Todo List",
                    ) {
                        TodoList
                    }
                }
            }
        }
    }
}

#[component]
fn Counter() -> Element {
    let count = create_state(0);

    layout! {
        FlexView {
            Text(computed!([count] || format!("Count: {}", count.get())))
            Button(
                .title = "Click",
                .on_click = callback!([count] || {
                    count.mutate(|count| *count += 1);
                })
            )
            if count.get() % 2 == 0 {
                Text("Is Even!")
            }
        }
    }
}

#[component]
fn TodoList() -> Element {
    let items = create_state::<Vec<(String, String)>>(Vec::new());
    let input_text = create_state("".to_string());

    let on_text_change = callback!([input_text] |text: &str| {
        input_text.set(text.to_string());
    });

    let add = callback!(
        [items, input_text] || {
            let text = input_text.get();
            if !text.is_empty() {
                items.mutate(|items| {
                    items.push((nanoid::nanoid!(), text));
                });
                input_text.set("".to_string());
            }
        }
    );

    let remove = callback!([items] |key: &str| {
        items.mutate(|items| {
            items.retain(|(k, _)| k != key);
        });
    });

    let move_up = callback!([items] |key: &str| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                if index > 0 {
                    items.swap(index, index - 1);
                }
            }
        });
    });

    let move_down = callback!([items] |key: &str| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                if index < items.len() - 1 {
                    items.swap(index, index + 1);
                }
            }
        });
    });

    let set_content = callback!([items] |key: &str, content: String| {
        items.mutate(|items| {
            if let Some(index) = items.iter().position(|(k, _)| k == key) {
                items[index] = (key.to_string(), content);
            }
        });
    });

    layout! {
        FlexView {
            FlexView(
                .direction = Direction::Row,
                .alignment = Alignment::Center
            ) {
                Input(
                    .value = input_text,
                    .grow = 1.0,
                    .on_text_change = on_text_change
                )
                Button(.title = "Add", .on_click = add)
            }
            FlexView(.grow = 1.0) {
                for item in items where key = |item| item.0.clone() {
                    TodoListItem(
                        .key = computed!([item] || item.get().0),
                        .content = computed!(move || item.get().1),
                        .remove = remove.clone(),
                        .move_up = move_up.clone(),
                        .move_down = move_down.clone(),
                        .set_content = set_content.clone(),
                    )
                }
            }
        }
    }
}

#[props]
struct TodoListItemProps {
    key: String,
    content: String,
    remove: Shared<dyn Fn(&str)>,
    move_up: Shared<dyn Fn(&str)>,
    move_down: Shared<dyn Fn(&str)>,
    set_content: Shared<dyn Fn(&str, String)>,
}

#[component]
fn TodoListItem(props: &TodoListItemProps) -> Element {
    let is_edit = create_state(false);

    let toggle_edit = callback!(
        [is_edit] || {
            is_edit.update(|is_edit| !is_edit);
        }
    );

    layout! {
        FlexView(.direction = Direction::Row) {
            Button(
                .title = "✕",
                .on_click = computed!([props.key, props.remove] || {
                    callback!([key: key.get(), remove: remove.get()] || remove(&key))
                })
            )
            Button(
                .title = "↑",
                .on_click = computed!([props.key, props.move_up] || {
                    callback!([key: key.get(), move_up: move_up.get()] || move_up(&key))
                })
            )
            Button(
                .title = "↓",
                .on_click = computed!([props.key, props.move_down] || {
                    callback!([key: key.get(), move_down: move_down.get()] || move_down(&key))
                })
            )
            Button(
                .title = "Edit",
                .on_click = toggle_edit
            )

            if is_edit.get() {
                Input(
                    .value = props.content.clone(),
                    .on_text_change = computed!([props.key, props.set_content] || {
                        callback!([key: key.get(), set_content: set_content.get()] |value: &str| {
                            set_content(&key, value.to_string());
                        })
                    }),
                    .grow = 1.0,
                )
            } else {
                Text(props.content.clone())
            }
        }
    }
}
