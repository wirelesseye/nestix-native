use env_logger::Env;
use nestix::{
    Element, callback, component, computed, create_state, layout, mount_root, unmount_root,
};
use nestix_native::{
    AlignItems, Button, FlexDirection, FlexView, FlexWrap, Material, MaterialSource, Root, Text,
    TitlebarMode, Window,
};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    mount_root(&layout! { MaterialsExample });
}

#[component]
fn MaterialsExample() -> Element {
    let (material, set_material) = create_state(Material::SIDEBAR);
    let (source, set_source) = create_state(MaterialSource::Automatic);

    let material_name = computed!(
        [material] || {
            let material = material.get();
            if material == Material::WINDOW {
                "Window"
            } else if material == Material::SIDEBAR {
                "Sidebar"
            } else if material == Material::CONTENT {
                "Content"
            } else {
                "Transient"
            }
        }
    );
    let source_name = computed!(
        [source]
            || match source.get() {
                MaterialSource::Automatic => "Automatic (within window for this area)",
                MaterialSource::BehindWindow => "Behind window",
                MaterialSource::WithinWindow => "Within window",
            }
    );

    layout! {
        Root {
            Window(
                .title = "Nestix Materials",
                .desktop(
                    .width = 760,
                    .height = 560,
                    .titlebar_mode = TitlebarMode::Overlay,
                    .material = Material::WINDOW,
                    .on_close_requested = callback!(|| {
                        unmount_root().expect("root should be mounted");
                    }),
                ),
            ) {
                FlexView(.view(.flex_grow = 1.0), .container(.padding = 24), .gap = 18) {
                    Text("Material preview")
                    Text(
                        computed!(
                            [material_name, source_name] || {
                                format!(
                                    "Material: {}  •  Source: {}",
                                    material_name.get(),
                                    source_name.get(),
                                )
                            }
                        ),
                    )
                    FlexView(
                        .view(.flex_grow = 1.0),
                        .container(.padding = 28),
                        .align_items = AlignItems::Center,
                        .gap = 12,
                        .material = material.clone(),
                        .material_source = source.clone(),
                    ) {
                        Text("This bounded area owns the selected material.")
                        Text("Compare within-window sampling with the desktop behind the window.")
                    }
                    Text("Material")
                    FlexView(
                        .flex_direction = FlexDirection::Row,
                        .flex_wrap = FlexWrap::Wrap,
                        .gap = 8,
                    ) {
                        Button(
                            .title = "Window",
                            .on_click = callback!(
                                [set_material] || {
                                    set_material.set(Material::WINDOW);
                                }
                            ),
                        )
                        Button(
                            .title = "Sidebar",
                            .on_click = callback!(
                                [set_material] || {
                                    set_material.set(Material::SIDEBAR);
                                }
                            ),
                        )
                        Button(
                            .title = "Content",
                            .on_click = callback!(
                                [set_material] || {
                                    set_material.set(Material::CONTENT);
                                }
                            ),
                        )
                        Button(
                            .title = "Transient",
                            .on_click = callback!(
                                [set_material] || {
                                    set_material.set(Material::TRANSIENT);
                                }
                            ),
                        )
                    }
                    Text("Material source")
                    FlexView(
                        .flex_direction = FlexDirection::Row,
                        .flex_wrap = FlexWrap::Wrap,
                        .gap = 8,
                    ) {
                        Button(
                            .title = "Automatic",
                            .on_click = callback!(
                                [set_source] || {
                                    set_source.set(MaterialSource::Automatic);
                                }
                            ),
                        )
                        Button(
                            .title = "Behind window",
                            .on_click = callback!(
                                [set_source] || {
                                    set_source.set(MaterialSource::BehindWindow);
                                }
                            ),
                        )
                        Button(
                            .title = "Within window",
                            .on_click = callback!(
                                [set_source] || {
                                    set_source.set(MaterialSource::WithinWindow);
                                }
                            ),
                        )
                    }
                }
            }
        }
    }
}
