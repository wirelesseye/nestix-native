use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use nestix::{
    Element, State, callback, closure, component, components::ContextProvider, create_state,
    layout, scoped_effect,
};
use nestix_native_core::{
    SelectOptionProps, SelectProps, StyleContext, dpi::LogicalSize, matched_style,
};
use windows::{
    Win32::{
        Foundation::{LPARAM, WPARAM},
        Graphics::Gdi::{DeleteObject, HFONT},
        UI::{
            Controls::{CB_SETMINVISIBLE, WC_COMBOBOX},
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, CBN_SELCHANGE,
                CBS_DROPDOWNLIST, CreateWindowExW, SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_COMMAND, WM_SETFONT, WS_CHILD, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
            },
        },
    },
    core::HSTRING,
};

use crate::{
    AppState, WindowContext, contexts::ParentContext, font::ui_font, native_control, utils::hiword,
};

static NEXT_OPTION_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone)]
struct OptionEntry {
    id: usize,
    label: String,
    value: String,
    enabled: bool,
}

#[derive(Clone)]
struct SelectContext {
    hwnd: windows::Win32::Foundation::HWND,
    options: Rc<RefCell<Vec<OptionEntry>>>,
    revision: State<usize>,
}

#[component]
pub fn Select(props: &SelectProps, element: &Element) -> Element {
    const DEFAULT_CLASSES: [&str; 2] = ["__Select", "__win32_Select"];
    let app_state = element.context::<AppState>().unwrap();
    let window = element.context::<WindowContext>().unwrap();
    let parent = element.context::<ParentContext>().unwrap();
    let style = matched_style(
        element.context::<StyleContext>(),
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_COMBOBOX,
            None,
            WS_VISIBLE | WS_CHILD | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            0,
            0,
            0,
            200,
            Some(parent.parent_hwnd),
            None,
            None,
            None,
        )
        .unwrap()
    };
    let intrinsic = create_state(LogicalSize::new(120.0, 28.0));
    native_control::mount(
        element,
        hwnd,
        style,
        &props.view,
        intrinsic.clone().into_readonly(),
    );

    let options = Rc::new(RefCell::new(Vec::<OptionEntry>::new()));
    let revision = create_state(0usize);
    unsafe {
        SendMessageW(hwnd, CB_SETMINVISIBLE, Some(WPARAM(8)), None);
    }
    app_state.add_control_handler(hwnd, callback!([options, props.value, props.on_value_change] |msg: u32, wparam: WPARAM, _: LPARAM| {
        if msg == WM_COMMAND && hiword(wparam.0 as _) as u32 == CBN_SELCHANGE {
            let index = unsafe { SendMessageW(hwnd, CB_GETCURSEL, None, None).0 };
            if index >= 0 {
                let option = options.borrow().get(index as usize).cloned();
                if let Some(option) = option {
                    if option.enabled {
                        if let Some(callback) = on_value_change.get() { callback(&option.value); }
                    } else {
                        select_value(hwnd, &options.borrow(), value.get().as_deref());
                    }
                }
            }
        }
    }));
    let font = Rc::new(Cell::new(None::<HFONT>));
    element.on_unmount(closure!(
        [app_state, font] || {
            app_state.remove_control_handler(hwnd);
            if let Some(font) = font.take() {
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    ));

    scoped_effect!(
        [props.enabled]
            || unsafe {
                let _ = EnableWindow(hwnd, enabled.get());
            }
    );
    scoped_effect!(
        [window.scale_factor, font]
            || unsafe {
                let next_font = ui_font(12.0, scale_factor.get());
                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(next_font.0 as _)),
                    Some(LPARAM(1)),
                );
                if let Some(previous) = font.replace(Some(next_font)) {
                    let _ = DeleteObject(previous.into());
                }
            }
    );
    scoped_effect!(
        [options, revision, props.value, intrinsic] || {
            let _ = revision.get();
            let options = options.borrow();
            select_value(hwnd, &options, value.get().as_deref());
            let width = options
                .iter()
                .map(|option| option.label.chars().count())
                .max()
                .unwrap_or(10);
            intrinsic.set(LogicalSize::new(
                (width as f32 * 7.0 + 40.0).max(120.0),
                28.0,
            ));
        }
    );

    layout! {
        ContextProvider<SelectContext>(SelectContext { hwnd, options, revision }) {
            $(props.children.clone())
        }
    }
}

#[component]
pub fn SelectOption(props: &SelectOptionProps, element: &Element) {
    let context = element.context::<SelectContext>().unwrap();
    let id = NEXT_OPTION_ID.fetch_add(1, Ordering::Relaxed);
    let initial_label = props.label.get();
    let initial_value = props.value.get();
    let initial_enabled = props.enabled.get();

    element.on_place(closure!(
        [context] | placement | {
            let mut options = context.options.borrow_mut();
            options.retain(|option| option.id != id);
            let index = placement.index.unwrap_or(options.len()).min(options.len());
            options.insert(
                index,
                OptionEntry {
                    id,
                    label: initial_label.clone(),
                    value: initial_value.clone(),
                    enabled: initial_enabled,
                },
            );
            drop(options);
            rebuild(&context);
        }
    ));
    element.on_unmount(closure!(
        [context] || {
            context
                .options
                .borrow_mut()
                .retain(|option| option.id != id);
            rebuild(&context);
        }
    ));
    scoped_effect!(
        [context, props.label, props.value, props.enabled] || {
            let changed = {
                let mut options = context.options.borrow_mut();
                if let Some(option) = options.iter_mut().find(|option| option.id == id) {
                    option.label = label.get();
                    option.value = value.get();
                    option.enabled = enabled.get();
                    true
                } else {
                    false
                }
            };
            if changed {
                rebuild(&context);
            }
        }
    );
}

fn rebuild(context: &SelectContext) {
    unsafe {
        SendMessageW(context.hwnd, CB_RESETCONTENT, None, None);
        for option in context.options.borrow().iter() {
            let label = HSTRING::from(&option.label);
            SendMessageW(
                context.hwnd,
                CB_ADDSTRING,
                None,
                Some(LPARAM(label.as_ptr() as isize)),
            );
        }
    }
    context
        .revision
        .mutate(|revision| *revision = revision.wrapping_add(1));
}

fn select_value(
    hwnd: windows::Win32::Foundation::HWND,
    options: &[OptionEntry],
    value: Option<&str>,
) {
    let index = value.and_then(|value| options.iter().position(|option| option.value == value));
    unsafe {
        SendMessageW(
            hwnd,
            CB_SETCURSEL,
            Some(WPARAM(index.unwrap_or(usize::MAX))),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlled_selection_resolves_first_matching_value() {
        let options = [
            OptionEntry {
                id: 1,
                label: String::new(),
                value: "first".into(),
                enabled: true,
            },
            OptionEntry {
                id: 2,
                label: String::new(),
                value: "duplicate".into(),
                enabled: true,
            },
            OptionEntry {
                id: 3,
                label: String::new(),
                value: "duplicate".into(),
                enabled: true,
            },
        ];
        assert_eq!(
            options
                .iter()
                .position(|option| option.value == "duplicate"),
            Some(1)
        );
        assert_eq!(
            options.iter().position(|option| option.value == "missing"),
            None
        );
    }
}
