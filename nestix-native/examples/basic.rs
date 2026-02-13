use env_logger::Env;
use nestix::{Element, component, layout, render_root};
use nestix_native::{Root, Label, StackView, Window};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    render_root(&layout! {ExampleApp});
}

#[component]
fn ExampleApp() -> Element {
    layout! {
        Root {
            Window(
                .title = "Counter",
                .width = 300,
                .height = 300,
                .view = layout! {
                    StackView {
                        Label(.text = "Hello")
                    }
                }
            )
        }
    }
}
