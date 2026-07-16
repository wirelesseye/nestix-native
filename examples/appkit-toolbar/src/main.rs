use env_logger::Env;
use nestix::{Element, callback, component, computed, create_state, layout, mount_root};
use nestix_native_appkit::{
    AppKitToolbar, AppKitToolbarDisplayMode, AppKitToolbarFlexibleSpace, AppKitToolbarItem,
    AppKitToolbarSpace, Button, FlexView, Root, Text, Window,
};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { AppKitToolbarExample });
}

#[component]
fn AppKitToolbarExample() -> Element {
    let count = create_state(0_i32);
    let reset_hidden = create_state(false);
    let display_mode = create_state(AppKitToolbarDisplayMode::IconAndLabel);

    layout! {
        Root {
            Window(
                .title = "AppKit Toolbar Example",
                .width = 560,
                .height = 320,
            ) {
                FlexView(.container(.padding = 15), .gap = 10) {
                    // AppKitToolbar may be mounted anywhere below its Window.
                    // It attaches to the window and takes no content-layout space.
                    AppKitToolbar(
                        .identifier = "dev.nestix.example.appkit-toolbar",
                        .display_mode = display_mode.clone(),
                    ) {
                        AppKitToolbarItem(
                            .identifier = "decrement",
                            .label = "Remove",
                            .symbol_name = Some("minus".to_string()),
                            .accessibility_description = Some("Remove one".to_string()),
                            .tool_tip = Some("Decrease the counter".to_string()),
                            .bordered = true,
                            .disabled = computed!([count] || count.get() <= 0),
                            .on_click = callback!([count] || {
                                count.mutate(|value| *value -= 1);
                            }),
                        )
                        AppKitToolbarSpace()
                        AppKitToolbarItem(
                            .identifier = "increment",
                            .label = "Add",
                            .symbol_name = Some("plus".to_string()),
                            .accessibility_description = Some("Add one".to_string()),
                            .tool_tip = Some("Increase the counter".to_string()),
                            .bordered = true,
                            .on_click = callback!([count] || {
                                count.mutate(|value| *value += 1);
                            }),
                        )
                        AppKitToolbarFlexibleSpace()
                        AppKitToolbarItem(
                            .identifier = "reset",
                            .label = "Reset",
                            .symbol_name = Some("arrow.counterclockwise".to_string()),
                            .accessibility_description = Some("Reset counter".to_string()),
                            .tool_tip = Some("Reset the counter to zero".to_string()),
                            .disabled = computed!([count] || count.get() == 0),
                            .hidden = reset_hidden.clone(),
                            .bordered = true,
                            .on_click = callback!([count] || count.set(0)),
                        )
                    }

                    Text(computed!([count] || format!("Count: {}", count.get())))
                    Button(
                        .title = computed!([reset_hidden] || if reset_hidden.get() {
                            "Show reset toolbar item"
                        } else {
                            "Hide reset toolbar item"
                        }),
                        .on_click = callback!([reset_hidden] || {
                            reset_hidden.mutate(|hidden| *hidden = !*hidden);
                        }),
                    )
                    Button(
                        .title = "Cycle toolbar display mode",
                        .on_click = callback!([display_mode] || {
                            display_mode.set(match display_mode.get() {
                                AppKitToolbarDisplayMode::Default => {
                                    AppKitToolbarDisplayMode::IconAndLabel
                                }
                                AppKitToolbarDisplayMode::IconAndLabel => {
                                    AppKitToolbarDisplayMode::IconOnly
                                }
                                AppKitToolbarDisplayMode::IconOnly => {
                                    AppKitToolbarDisplayMode::LabelOnly
                                }
                                AppKitToolbarDisplayMode::LabelOnly => {
                                    AppKitToolbarDisplayMode::Default
                                }
                            });
                        }),
                    )
                }
            }
        }
    }
}
