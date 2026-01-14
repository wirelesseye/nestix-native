use nestix::{Element, component, components::ContextProvider, layout};
use nestix_native_core::StackViewProps;
use windows::{Win32::UI::WindowsAndMessaging::{CreateWindowExW, WINDOW_EX_STYLE, WS_CHILD, WS_VISIBLE}, core::w};

use crate::ParentContext;

#[component]
pub fn StackView(props: &StackViewProps, element: &Element) -> Element {
    let parent = element.context::<ParentContext>().unwrap();
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            None,
            WS_VISIBLE | WS_CHILD,
            0,
            0,
            200,
            25,
            parent.hwnd,
            None,
            None,
            None,
        ).unwrap()
    };

    layout! {
        ContextProvider<ParentContext>(
            .value = ParentContext {
                hwnd: Some(hwnd)
            },
            .children = props.children.clone(),
        )
    }
}
