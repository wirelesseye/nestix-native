#[cfg(target_os = "macos")]
fn main() {
    app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod app {
    use env_logger::Env;
    use nestix::{
        Element, callback, component, computed, create_state, layout, mount_root, unmount_root, props,
    };
    use nestix_native::{Button, FlexView, LayoutInspector, Root, Text, Window};

    pub fn run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
        mount_root(&layout! { ExampleApp });
    }

    #[component]
    fn ExampleApp() -> Element {
        let (count, set_count) = create_state(0_u32);
        let (items, set_items) = create_state(vec![(0_u32, "First item".to_string())]);
        let (inspector_visible, set_inspector_visible) = create_state(true);

        layout! {
            Root {
                Window(
                    .title = "Inspector Example",
                    .desktop(
                        .on_close_requested = callback!(|| {
                            unmount_root().expect("root should be mounted");
                        }),
                    ),
                ) {
                    FlexView(.view(.flex_grow = 1.0)) {
                        Text("Application window")
                        Counter(.count = count.clone())
                        if count.get() % 2 == 0 {
                            InternalWrapper
                        }
                        for item in items where key = |item| item.0 {
                            ListItem(.item = item)
                        }
                        Button(
                            .title = "Increment",
                            .on_click = callback!(
                                [set_count] || {
                                    set_count.update(|count| count + 1);
                                }
                            ),
                        )
                        Button(
                            .title = "Add list item",
                            .on_click = callback!(
                                [set_items] || {
                                    set_items.mutate(|items| {
                                        let id = items.len() as u32;
                                        items.push((id, format!("Item {}", id + 1)));
                                    });
                                }
                            ),
                        )
                        Button(
                            .title = "Show inspector",
                            .on_click = callback!(
                                [set_inspector_visible] || {
                                    set_inspector_visible.set(true);
                                }
                            ),
                        )
                    }
                }
                LayoutInspector(
                    .visible = inspector_visible,
                    .on_close_requested = callback!(
                        [set_inspector_visible] || {
                            set_inspector_visible.set(false);
                        }
                    ),
                )
            }
        }
    }

    #[props]
    struct CounterProps {
        count: u32,
    }

    #[component]
    fn Counter(props: &CounterProps) -> Element {
        layout! {
            Text(computed!([props.count] || format!("Count: {}", count.get())))
        }
    }

    #[props]
    struct ListItemProps {
        item: (u32, String),
    }

    #[component]
    fn ListItem(props: &ListItemProps) -> Element {
        layout! {
            Text(computed!([props.item] || item.get().1))
        }
    }

    #[component(internal)]
    fn InternalWrapper() -> Element {
        layout! {
            Text("Visible child promoted through an internal component")
        }
    }
}
