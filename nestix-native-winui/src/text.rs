use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::TextProps;

use crate::{contexts::ParentContext, xaml::XamlElement};

#[component]
pub fn Text(props: &TextProps, element: &Element) {
    let parent_context = element.context::<ParentContext>().unwrap();
    let text_block =
        XamlElement::text_block(props.text.get()).expect("failed to create WinUI TextBlock");
    element.provide_handle(text_block.clone());

    let placed_text = text_block.clone();
    element.on_place(closure!(
        [parent_context] | _ | {
            (parent_context.add_child)(placed_text.clone());
        }
    ));

    let unmount_text = text_block.clone();
    element.on_unmount(closure!(
        [parent_context] || {
            (parent_context.remove_child)(&unmount_text);
        }
    ));

    let effect_text = text_block.clone();
    scoped_effect!(
        element,
        [props.text] || {
            let _ = effect_text.set_text(text.get());
        }
    );
}
