use nestix::{Element, callback, component, computed, create_model, create_state, layout};
use nestix_native_appkit::{
    app::AppkitApp, button::AppkitButton, label::AppkitLabel, stack_view::AppkitStackView,
    window::AppkitWindow,
};

fn main() {
    let model = create_model();
    let app = layout! {App};
    model.render(&app);
}

#[component]
fn App() -> Element {
    let count = create_state(0);

    layout! {
        AppkitApp(
            .should_terminate_after_last_window_closed = true
        ) {
            AppkitWindow(
                .title = "Counter",
                .width = 300,
                .height = 300,
                .view = layout! {
                    AppkitStackView {
                        AppkitLabel(.text = computed!(count => || format!("Count: {}", count.get())))
                        AppkitButton(
                            .title = "Click",
                            .y = 20,
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
