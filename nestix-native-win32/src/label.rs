use nestix::{Element, component};
use nestix_native_core::LabelProps;
use windows::{
    Win32::UI::WindowsAndMessaging::{CreateWindowExW, WINDOW_EX_STYLE, WS_CHILD, WS_VISIBLE},
    core::{HSTRING, w},
};

use crate::ParentContext;

#[component]
pub fn Label(props: &LabelProps, element: &Element) {
    let parent = element.context::<ParentContext>().unwrap();

    let text = HSTRING::from(props.text.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            &text,
            WS_VISIBLE | WS_CHILD,
            props.x.get() as i32,
            props.y.get() as i32,
            200,
            25,
            parent.hwnd,
            None,
            None,
            None,
        )
        .unwrap()
    };
}
