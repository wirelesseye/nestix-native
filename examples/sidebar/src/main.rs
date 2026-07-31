use env_logger::Env;
use nestix::{Element, callback, component, layout, mount_root, unmount_root};
use nestix_native::{FlexView, Input, Root, Sidebar, Text, TitlebarMode, Window};

#[cfg(target_os = "macos")]
use nestix_native::appkit::{AppKitToolbar, AppKitToolbarStyle};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { ExampleApp });
}

#[component]
fn ExampleApp() -> Element {
    layout! {
        Root {
            Window(
                .title = "Nestix Sidebar",
                .desktop(
                    .width = 800,
                    .height = 520,
                    .on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    }),
                    .titlebar_mode = TitlebarMode::Overlay,
                ),
            ) {
                FlexView(.view(.flex_grow = 1.0)) {
                    #[cfg(target_os = "macos")]
                    AppKitToolbar(
                        .identifier = "dev.nestix.example.sidebar",
                        .style = AppKitToolbarStyle::Unified,
                    )
                    Sidebar(.width = 260.0, .min_width = 260.0, .resizable = true) {
                        FlexView(
                            .container(
                                .padding_horizontal = 15,
                                .padding_bottom = 15,
                                .padding_top = 50,
                            ),
                            .gap = 15,
                        ) {
                            Input()
                            Text("Sidebar")
                            Text("Navigation and tools go here.")
                        }
                    }
                    FlexView(.view(.flex_grow = 1.0)) {
                        Text("Main content")
                    }
                }
            }
        }
    }
}
