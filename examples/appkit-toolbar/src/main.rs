use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    Button, FlexView, Length, Root, StyleProvider, Text, TitleBarMode, Window,
    appkit::{
        AppKitToolbar, AppKitToolbarDisplayMode, AppKitToolbarFlexibleSpace, AppKitToolbarItem,
        AppKitToolbarSpace, AppKitToolbarStyle,
    },
    computed_style,
};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { AppKitToolbarExample });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExamplePage {
    Counter,
    Appearance,
}

#[component]
fn AppKitToolbarExample() -> Element {
    let count = create_state(0_i32);
    let reset_hidden = create_state(false);
    let page = create_state(ExamplePage::Counter);
    let display_mode = create_state(AppKitToolbarDisplayMode::IconAndLabel);
    let toolbar_style = create_state(AppKitToolbarStyle::Expanded);
    let title_bar_mode = create_state(TitleBarMode::System);

    let selected_identifier = computed!(
        [page] || {
            Some(
                match page.get() {
                    ExamplePage::Counter => "counter",
                    ExamplePage::Appearance => "appearance",
                }
                .to_string(),
            )
        }
    );

    let padding_top = computed!(
        [title_bar_mode]
            || if title_bar_mode.get() == TitleBarMode::Overlay {
                Length::logical(80).into()
            } else {
                Length::logical(15).into()
            }
    );

    let styles = computed_style!(
        []

        .content {
            padding_top: $(padding_top.get());
            padding_bottom: 15 px;
            padding_horizontal: 15 px;
            gap: 10 px;
        }
    );
    let toolbar_page = page.clone();

    layout! {
        Root {
            StyleProvider(styles) {
                Window(
                    .title = "AppKit Toolbar Example",
                    .desktop(
                        .width = 560,
                        .height = 320,
                        .title_bar_mode = title_bar_mode.clone(),
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    ),
                ) {
                    FlexView(.class = "content") {
                        // AppKitToolbar may be mounted anywhere below its Window.
                        // It attaches to the window and takes no content-layout space.
                        AppKitToolbar(
                            .identifier = "dev.nestix.example.appkit-toolbar",
                            .selected_identifier = selected_identifier,
                            .display_mode = display_mode.clone(),
                            .style = toolbar_style.clone(),
                        ) {
                            AppKitToolbarItem(
                                .identifier = "counter",
                                .label = "Counter",
                                .symbol_name = Some("number".to_string()),
                                .accessibility_description = Some("Counter page".to_string()),
                                .selectable = true,
                                .on_click = callback!(
                                    [page] || page.set(ExamplePage::Counter)
                                ),
                            )
                            AppKitToolbarItem(
                                .identifier = "appearance",
                                .label = "Appearance",
                                .symbol_name = Some("paintbrush".to_string()),
                                .accessibility_description = Some("Appearance page".to_string()),
                                .selectable = true,
                                .on_click = callback!(
                                    [page] || page.set(ExamplePage::Appearance)
                                ),
                            )
                            AppKitToolbarFlexibleSpace()
                            if toolbar_page.get() == ExamplePage::Counter {
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
                                AppKitToolbarSpace()
                                AppKitToolbarItem(
                                    .identifier = "decrement",
                                    .label = "Remove",
                                    .symbol_name = Some("minus".to_string()),
                                    .accessibility_description = Some("Remove one".to_string()),
                                    .tool_tip = Some("Decrease the counter".to_string()),
                                    .bordered = true,
                                    .disabled = computed!([count] || count.get() <= 0),
                                    .on_click = callback!(
                                        [count] || {
                                            count.mutate(|value| *value -= 1);
                                        }
                                    ),
                                )
                                AppKitToolbarItem(
                                    .identifier = "increment",
                                    .label = "Add",
                                    .symbol_name = Some("plus".to_string()),
                                    .accessibility_description = Some("Add one".to_string()),
                                    .tool_tip = Some("Increase the counter".to_string()),
                                    .bordered = true,
                                    .on_click = callback!(
                                        [count] || {
                                            count.mutate(|value| *value += 1);
                                        }
                                    ),
                                )
                            }
                        }
                        if page.get() == ExamplePage::Counter {
                            Text(computed!([count] || format!("Count: {}", count.get())))
                            Button(
                                .title = computed!(
                                    [reset_hidden]
                                        || if reset_hidden.get() {
                                            "Show reset toolbar item"
                                        } else {
                                            "Hide reset toolbar item"
                                        }
                                ),
                                .on_click = callback!(
                                    [reset_hidden] || {
                                        reset_hidden.mutate(|hidden| *hidden = !*hidden);
                                    }
                                ),
                            )
                        } else {
                            Button(
                                .title = "Cycle toolbar display mode",
                                .on_click = callback!(
                                    [display_mode] || {
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
                                    }
                                ),
                            )
                            Button(
                                .title = "Cycle toolbar style",
                                .on_click = callback!(
                                    [toolbar_style] || {
                                        toolbar_style.set(match toolbar_style.get() {
                                            AppKitToolbarStyle::Automatic => {
                                                AppKitToolbarStyle::Expanded
                                            }
                                            AppKitToolbarStyle::Expanded => {
                                                AppKitToolbarStyle::Preference
                                            }
                                            AppKitToolbarStyle::Preference => {
                                                AppKitToolbarStyle::Unified
                                            }
                                            AppKitToolbarStyle::Unified => {
                                                AppKitToolbarStyle::UnifiedCompact
                                            }
                                            AppKitToolbarStyle::UnifiedCompact => {
                                                AppKitToolbarStyle::Automatic
                                            }
                                        });
                                    }
                                ),
                            )
                            Button(
                                .title = computed!(
                                    [title_bar_mode]
                                        || format!(
                                            "Toggle title bar overlay mode (current: {:?})",
                                            title_bar_mode.get(),
                                        )
                                ),
                                .on_click = callback!(
                                    [title_bar_mode] || {
                                        title_bar_mode.set(match title_bar_mode.get() {
                                            TitleBarMode::System => TitleBarMode::Overlay,
                                            TitleBarMode::Overlay => TitleBarMode::System,
                                            _ => TitleBarMode::System,
                                        });
                                    }
                                ),
                            )
                        }
                    }
                }
            }
        }
    }
}
