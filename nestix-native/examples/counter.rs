use nestix::{Element, callback, component, computed, create_state, layout, render};
use nestix_native::{App, Button, Label, ListView, Window};

fn main() {
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
        }
    }
}
