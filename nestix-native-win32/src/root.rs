use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    ffi::c_void,
    rc::Rc,
};

use nestix::{Element, PropValue, closure, component, components::ContextProvider, layout};
use nestix_native_core::RootProps;
use windows::Win32::{
    Foundation::HWND,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage},
    },
};

use crate::window::WindowState;

thread_local! {
    static APP_STATE: OnceCell<Rc<AppState>> = OnceCell::new();
}

pub(crate) fn shared_app_state() -> Rc<AppState> {
    APP_STATE.with(|app| app.get().unwrap().clone())
}

pub(crate) struct AppState {
    is_running: Cell<bool>,
    windows: RefCell<HashMap<*mut c_void, Rc<WindowState>>>,
    quit_when_all_windows_closed: PropValue<bool>,
}

impl AppState {
    fn new(props: &RootProps) -> Self {
        Self {
            is_running: Cell::new(false),
            windows: RefCell::new(HashMap::new()),
            quit_when_all_windows_closed: props.quit_when_all_windows_closed.clone(),
        }
    }

    fn run(&self) {
        self.is_running.set(true);

        let mut msg = MSG::default();
        unsafe {
            while self.is_running.get() {
                if GetMessageW(&mut msg, None, 0, 0).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    pub fn quit(&self) {
        self.is_running.set(false);
    }

    pub fn quit_when_all_windows_closed(&self) -> bool {
        self.quit_when_all_windows_closed.get()
    }

    pub fn has_windows(&self) -> bool {
        !self.windows.borrow().is_empty()
    }

    pub(crate) fn add_window(&self, window: HWND, state: Rc<WindowState>) {
        self.windows.borrow_mut().insert(window.0, state);
    }

    pub(crate) fn window_state(&self, window: HWND) -> Option<Rc<WindowState>> {
        self.windows.borrow().get(&window.0).cloned()
    }

    pub(crate) fn remove_window(&self, window: HWND) {
        self.windows.borrow_mut().remove(&window.0);
    }
}

#[component]
pub fn Root(props: &RootProps, element: &Element) -> Element {
    let app_state = APP_STATE.with(|app| app.get_or_init(|| Rc::new(AppState::new(props))).clone());

    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).unwrap();
    }

    element.after_render(closure!(
        [app_state] || {
            app_state.run();
        }
    ));

    layout! {
        ContextProvider<AppState>(
            .value = app_state,
        ) {
            $(props.children.clone())
        }
    }
}
