use env_logger::Env;
use nestix::{Element, callback, component, computed, create_state, layout, render};
use nestix_native::{App, Button, Label, ListView, TabView, TabViewItem, Window};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    render(&layout! {CounterApp});
}

#[component]
fn CounterApp() -> Element {
    let count = create_state(0);

    layout! {
        App {
            Window(
                .title = "Counter",
                .width = 300,
                .height = 300,
                .view = layout! {
                    TabView {
                        TabViewItem(
                            .id = "counter",
                            .title = "Counter",
                            .view = layout! {
                                ListView {
                                    Label(.text = computed!(count => || format!("Count: {}", count.get())))
                                    Button(
                                        .title = "Click",
                                        .on_click = callback!(count => || {
                                            count.mutate(|count| *count += 1);
                                        })
                                    )
                                }
                            }
                        )
                        TabViewItem(
                            .id = "todo_list",
                            .title = "Todo List",
                            .view = layout! {
                                ListView {
                                    Label(.text = "Todo List")
                                }
                            }
                        )
                    }
                }
            )
        }
    }
}
