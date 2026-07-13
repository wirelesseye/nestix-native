use std::{
    cell::RefCell,
    ffi::c_void,
    ptr::{null, null_mut},
    rc::Rc,
    sync::{Once, OnceLock},
};

use nestix::{Element, closure, component, scoped_effect};
use nestix_native_core::{
    ContentFit, Dimension, ImageSource, ImageViewProps, StyleContext, TreeContext,
    dpi::{LogicalPosition, LogicalSize},
    matched_style, style_align_self, style_dimension, style_flex_basis, style_flex_grow,
    style_flex_shrink, style_margin,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{
    Size, Style,
    prelude::{FromLength, FromPercent, TaffyAuto},
};
use windows::{
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Gdi::{
                BeginPaint, COLOR_BTNFACE, EndPaint, FillRect, GetSysColorBrush, InvalidateRect,
                PAINTSTRUCT,
            },
            GdiPlus::*,
        },
        System::{Com::IStream, LibraryLoader::GetModuleHandleW},
        UI::{Shell::SHCreateMemStream, WindowsAndMessaging::*},
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::{WindowContext, contexts::ParentContext};

struct ImageState {
    image: *mut GpImage,
    stream: Option<IStream>,
    width: u32,
    height: u32,
    fit: ContentFit,
}

impl Default for ImageState {
    fn default() -> Self {
        Self {
            image: null_mut(),
            stream: None,
            width: 0,
            height: 0,
            fit: ContentFit::Contain,
        }
    }
}

impl Drop for ImageState {
    fn drop(&mut self) {
        if !self.image.is_null() {
            unsafe {
                GdipDisposeImage(self.image);
            }
        }
    }
}

fn ensure_gdiplus() {
    static TOKEN: OnceLock<usize> = OnceLock::new();
    TOKEN.get_or_init(|| unsafe {
        let mut token = 0;
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        GdiplusStartup(&mut token, &input, null_mut());
        token
    });
}

fn image_classname(hinstance: HMODULE) -> PCWSTR {
    const NAME: PCWSTR = w!("NestixNativeImageView");
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        RegisterClassW(&WNDCLASSW {
            hInstance: hinstance.into(),
            lpszClassName: NAME,
            lpfnWndProc: Some(image_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            ..Default::default()
        });
    });
    NAME
}

unsafe fn window_state(hwnd: HWND) -> Option<Rc<RefCell<ImageState>>> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<ImageState>;
    if ptr.is_null() {
        return None;
    }
    unsafe {
        Rc::increment_strong_count(ptr);
    }
    Some(unsafe { Rc::from_raw(ptr) })
}

extern "system" fn image_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCREATE => {
                let cs = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
                return LRESULT(1);
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                FillRect(hdc, &ps.rcPaint, GetSysColorBrush(COLOR_BTNFACE));
                if let Some(state) = window_state(hwnd) {
                    let state = state.borrow();
                    if !state.image.is_null() && state.width > 0 && state.height > 0 {
                        let mut client = RECT::default();
                        GetClientRect(hwnd, &mut client).unwrap();
                        let cw = (client.right - client.left).max(0);
                        let ch = (client.bottom - client.top).max(0);
                        let iw = state.width as f32;
                        let ih = state.height as f32;
                        let (dw, dh) = match state.fit {
                            ContentFit::Fill => (cw as f32, ch as f32),
                            ContentFit::None => (iw, ih),
                            ContentFit::Contain => {
                                let s = (cw as f32 / iw).min(ch as f32 / ih);
                                (iw * s, ih * s)
                            }
                            ContentFit::Cover => {
                                let s = (cw as f32 / iw).max(ch as f32 / ih);
                                (iw * s, ih * s)
                            }
                            ContentFit::ScaleDown => {
                                let s = 1.0_f32.min((cw as f32 / iw).min(ch as f32 / ih));
                                (iw * s, ih * s)
                            }
                        };
                        let x = ((cw as f32 - dw) / 2.0).round() as i32;
                        let y = ((ch as f32 - dh) / 2.0).round() as i32;
                        let mut graphics = null_mut();
                        if GdipCreateFromHDC(hdc, &mut graphics) == Ok {
                            GdipSetInterpolationMode(graphics, InterpolationModeHighQualityBicubic);
                            GdipDrawImageRectRectI(
                                graphics,
                                state.image,
                                x,
                                y,
                                dw.round() as i32,
                                dh.round() as i32,
                                0,
                                0,
                                state.width as i32,
                                state.height as i32,
                                UnitPixel,
                                null(),
                                0,
                                null_mut(),
                            );
                            GdipDeleteGraphics(graphics);
                        }
                    }
                }
                EndPaint(hwnd, &ps).unwrap();
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<ImageState>;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if !ptr.is_null() {
                    drop(Rc::from_raw(ptr));
                }
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn load_image(source: ImageSource) -> (*mut GpImage, Option<IStream>, u32, u32) {
    ensure_gdiplus();
    unsafe {
        let mut image = null_mut();
        let stream = match source {
            ImageSource::File(path) => {
                let path = HSTRING::from(path.as_os_str());
                GdipLoadImageFromFile(&path, &mut image);
                None
            }
            ImageSource::Bytes(bytes) => {
                let stream = SHCreateMemStream(Some(&bytes));
                if let Some(stream) = &stream {
                    GdipLoadImageFromStream(stream, &mut image);
                }
                stream
            }
        };
        if image.is_null() {
            return (image, stream, 0, 0);
        }
        let mut width = 0;
        let mut height = 0;
        GdipGetImageWidth(image, &mut width);
        GdipGetImageHeight(image, &mut height);
        (image, stream, width, height)
    }
}

#[component]
pub fn ImageView(props: &ImageViewProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__ImageView", "__win32_ImageView"];
    let window_context = element.context::<WindowContext>().unwrap();
    let tree_context = element.context::<TreeContext>().unwrap();
    let parent_context = element.context::<ParentContext>().unwrap();
    let style_context = element.context::<StyleContext>();
    let style_props = matched_style(
        style_context,
        element,
        props.class.clone(),
        &DEFAULT_CLASSES,
    );
    let state = Rc::new(RefCell::new(ImageState::default()));
    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };
    let raw_state = Rc::into_raw(state.clone()) as *mut c_void;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            image_classname(hinstance),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            0,
            0,
            0,
            0,
            Some(parent_context.parent_hwnd),
            None,
            Some(hinstance.into()),
            Some(raw_state),
        )
        .unwrap()
    };
    element.provide_handle(hwnd);
    let node_id = tree_context.create_node(true);

    element.on_place(closure!(
        [parent_context] | placement | {
            if let Some(index) = placement.index
                && let Some(insert) = &parent_context.insert_child
            {
                insert(hwnd, Some(node_id), index);
            } else if let Some(add) = &parent_context.add_child {
                add(hwnd, Some(node_id));
            }
        }
    ));
    element.on_unmount(closure!(
        [parent_context] || {
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove) = &parent_context.remove_child {
                remove(hwnd, Some(node_id));
            }
        }
    ));

    scoped_effect!(
        element,
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style.as_ref(), flex_shrink.get()),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            parent_context.parent_node,
            tree_context,
            style_props,
            state,
            props.source,
            props.content_fit,
            props.view.width,
            props.view.height
        ] || {
            let (image, stream, iw, ih) = load_image(source.get());
            {
                let mut current = state.borrow_mut();
                if !current.image.is_null() {
                    unsafe {
                        GdipDisposeImage(current.image);
                    }
                }
                current.image = image;
                current.stream = stream;
                current.width = iw;
                current.height = ih;
                current.fit = content_fit.get();
            }
            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, true);
            }
            let sf = scale_factor.get();
            let style = style_props.get();
            let width = style_dimension(style.as_ref(), width.get(), Dimension::Auto, |s| s.width);
            let height =
                style_dimension(style.as_ref(), height.get(), Dimension::Auto, |s| s.height);
            let wa = width.is_auto();
            let ha = height.is_auto();
            let ratio = if ih > 0 { iw as f32 / ih as f32 } else { 1.0 };
            let (width, height) = match (width, height) {
                (Dimension::Auto, Dimension::Auto) => {
                    (iw as f32 / sf as f32, ih as f32 / sf as f32)
                }
                (Dimension::Length(w), Dimension::Auto) => {
                    let w = w.to_logical::<f32>(sf).0;
                    (w, w / ratio)
                }
                (Dimension::Auto, Dimension::Length(h)) => {
                    let h = h.to_logical::<f32>(sf).0;
                    (h * ratio, h)
                }
                (Dimension::Length(w), Dimension::Length(h)) => {
                    (w.to_logical::<f32>(sf).0, h.to_logical::<f32>(sf).0)
                }
            };
            if parent_node.is_some() {
                tree_context.update_style(node_id, |prev| Style {
                    size: Size {
                        width: taffy::Dimension::from_length(width),
                        height: taffy::Dimension::from_length(height),
                    },
                    max_size: Size {
                        width: if wa {
                            taffy::Dimension::from_percent(1.0)
                        } else {
                            taffy::Dimension::AUTO
                        },
                        height: if ha {
                            taffy::Dimension::from_percent(1.0)
                        } else {
                            taffy::Dimension::AUTO
                        },
                    },
                    item_is_replaced: true,
                    aspect_ratio: Some(ratio),
                    ..prev
                });
            }
            tree_context.refresh();
        }
    );

    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let sf = scale_factor.get();
            let style = style_props.get();
            let left = style_dimension(style.as_ref(), left.get(), Dimension::Auto, |s| s.left);
            let top = style_dimension(style.as_ref(), top.get(), Dimension::Auto, |s| s.top);
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, sf),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        element,
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style.as_ref(), margin.get()),
                    scale_factor.get(),
                ),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        element,
        [tree_context, style_props, props.view.align_self] || {
            let style = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });
            tree_context.refresh();
        }
    );
    scoped_effect!(
        element,
        [window_context.scale_factor, tree_context] || {
            if let Some(layout) = tree_context.layout(node_id) {
                let point = LogicalPosition::new(layout.location.x, layout.location.y)
                    .to_physical(scale_factor.get());
                let size = LogicalSize::new(layout.size.width, layout.size.height)
                    .to_physical(scale_factor.get());
                unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        point.x,
                        point.y,
                        size.width,
                        size.height,
                        SWP_NOZORDER,
                    )
                    .unwrap();
                    let _ = InvalidateRect(Some(hwnd), None, true);
                }
            }
        }
    );
}
