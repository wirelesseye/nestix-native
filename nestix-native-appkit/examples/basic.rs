use nestix::{Element, callback, component, computed, create_state, layout, render};
use nestix_native_appkit::{
    AppkitTabView, AppkitTabViewItem, app::AppkitApp, button::AppkitButton, label::AppkitLabel,
    list_view::AppkitListView, window::AppkitWindow,
};

fn main() {
    render(&layout! {CounterApp});
}

#[component]
fn CounterApp() -> Element {
    let count = create_state(0);

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
                            .view = layout! {
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
                        )
                        AppkitTabViewItem(
                            .id = "todo_list",
                            .title = "Todo List",
                            .view = layout! {
                                AppkitListView {
                                    AppkitLabel(.text = "Todo List")
                                }
                            }
                        )
                    }
                }
            )
        }
    }
}
