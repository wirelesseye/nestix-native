use env_logger::Env;
use nestix::{Element, callback, component, layout, render_root};
use nestix_native::{Button, Direction, FlexView, Label, Root, Window, Wrap, dpi};

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
                .on_resize = callback!(|size: dpi::Size| {
                    println!("{:?}", size);
                }),
            ) {
                FlexView(
                    .direction = Direction::Row,
                    .wrap = Wrap::Wrap,
                ) {
                    Label(.text = "Hello1")
                    Label(.text = "Hello2")
                    Button(.title = "Click me!")
                    Label(.text = "Hello3")
                }
            }
        }
    }
}
