use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::ButtonProps;

use crate::{contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    let parent_context = element.context::<ParentContext>().unwrap();
    let button = XamlElement::button(props.title.get()).expect("failed to create WinUI Button");
    element.provide_handle(button.clone());

    let placed_button = button.clone();
    element.on_place(closure!(
        [parent_context] | _ | {
            (parent_context.add_child)(placed_button.clone());
        }
    ));

    let unmount_button = button.clone();
    element.on_unmount(closure!(
        [parent_context] || {
            (parent_context.remove_child)(&unmount_button);
        }
    ));

    let title_button = button.clone();
    scoped_effect!(
        element,
        [props.title] || {
            let _ = title_button.set_text(title.get());
        }
    );

    let click_button = button.clone();
    scoped_effect!(
        element,
        [props.on_click] || {
            let _ = click_button.set_button_click(on_click.get());
        }
    );
}
