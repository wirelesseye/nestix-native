use env_logger::Env;
use nestix::{Element, callback, component, create_state, layout, mount_root};
use nestix_native::{Button, FlexDirection, FlexView, Root, Text, Window, FlexWrap};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! {ExampleApp});
}

#[component]
fn ExampleApp() -> Element {
    let text = create_state("Hello!".to_string());

    layout! {
        Root {
            Window(
                .title = "Counter",
                .width = 300,
                .height = 300,
                .on_resize = callback!(|size| {
                    println!("{:?}", size);
                }),
            ) {
                FlexView(
                    .flex_direction = FlexDirection::Row,
                    .flex_wrap = FlexWrap::Wrap,
                ) {
                    Text(text.clone())
                    Text("Hello2")
                    Button(
                        .title = "Click me!",
                        .on_click = callback!([] || {
                            text.mutate(|text| text.push_str("Hello!"));
                        })
                    )
                    Text("Hello3")
                }
            }
        }
    }
}
