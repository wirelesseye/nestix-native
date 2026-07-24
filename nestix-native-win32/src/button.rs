use nestix::{Element, callback, closure, component, scoped_effect};
use nestix_native_core::{
    Appearance, ButtonProps, Dimension, Rect, StyleContext, TreeContext,
    dpi::{LogicalPosition, LogicalSize, LogicalUnit, PhysicalUnit},
    matched_style, resolve_font_props, style_align_self, style_appearance, style_dimension,
    style_flex_basis, style_flex_grow, style_flex_shrink, style_margin, style_padding_with_default,
    utils::{inset_to_taffy, margin_to_taffy},
};
use taffy::{Size, Style, prelude::FromLength};
use windows::{
    Win32::{
        Foundation::{LPARAM, SIZE, WPARAM},
        Graphics::Gdi::{
            COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, DFC_BUTTON, DFCS_BUTTONPUSH,
            DFCS_INACTIVE, DFCS_PUSHED, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject,
            DrawFocusRect, DrawFrameControl, DrawTextW, FillRect, GetDC, GetSysColor,
            GetSysColorBrush, GetTextExtentPoint32W, HFONT, InflateRect, InvalidateRect,
            OffsetRect, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
        },
        UI::{
            Controls::{
                DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, SetWindowTheme, WC_BUTTON,
            },
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                BN_CLICKED, BS_OWNERDRAW, BS_PUSHBUTTON, BS_TYPEMASK, CreateWindowExW,
                DestroyWindow, GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOMOVE,
                SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetWindowLongPtrW, SetWindowPos,
                SetWindowTextW, WINDOW_EX_STYLE, WM_COMMAND, WM_DRAWITEM, WM_GETFONT, WM_SETFONT,
                WS_CHILD, WS_VISIBLE,
            },
        },
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::{AppState, WindowContext, contexts::ParentContext, font::resolved_font, utils::hiword};

const DEFAULT_PADDING_X: f32 = 10.0;
const DEFAULT_PADDING_Y: f32 = 3.0;

#[component]
pub fn Button(props: &ButtonProps, element: &Element) {
    const DEFAULT_CLASSES: [&str; 2] = ["__Button", "__win32_Button"];

    let app_state = element.context::<AppState>().unwrap();
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

    let title = HSTRING::from(props.title.get());
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WC_BUTTON,
            &title,
            WS_VISIBLE | WS_CHILD,
            0,
            0,
            0,
            0,
            Some(parent_context.parent_hwnd),
            None,
            None,
            None,
        )
        .unwrap()
    };
    element.provide_handle(hwnd);

    let node_id = tree_context.create_node(false);
    element.on_place(closure!(
        [parent_context] | placement | {
            parent_context.place_child(hwnd, Some(node_id), placement);
        }
    ));

    app_state.add_control_handler(
        hwnd,
        callback!([app_state, props.on_click, props.title] |msg: u32, wparam: WPARAM, lparam: LPARAM| {
            match msg {
                WM_COMMAND => {
                    if let Some(on_click) = on_click.get() {
                        if hiword(wparam.0 as _) as u32 == BN_CLICKED  {
                            on_click();
                        }
                    }
                },
                WM_DRAWITEM => unsafe {
                    let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
                    draw_button(
                        item,
                        &title.get(),
                        app_state.control_text_color(hwnd),
                    );
                },
                _ => (),
            }
        }),
    );

    scoped_effect!(
        [props.disabled]
            || unsafe {
                let _ = EnableWindow(hwnd, !disabled.get());
            }
    );

    element.on_unmount(closure!(
        [parent_context, app_state] || {
            unsafe {
                DestroyWindow(hwnd).unwrap();
            }
            if let Some(remove_child) = &parent_context.remove_child {
                remove_child(hwnd, Some(node_id));
            }
            app_state.remove_control_handler(hwnd);
            app_state.set_control_text_color(hwnd, None);
        }
    ));

    scoped_effect!(
        [
            tree_context,
            style_props,
            props.view.flex_grow,
            props.view.flex_basis,
            props.view.flex_shrink,
            window_context.scale_factor
        ] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                flex_grow: style_flex_grow(style_props.as_ref(), flex_grow.get()),
                flex_basis: style_flex_basis(style_props.as_ref(), flex_basis.get())
                    .to_taffy(scale_factor.get()),
                flex_shrink: style_flex_shrink(style_props.as_ref(), flex_shrink.get()),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            style_props,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            props.appearance
        ] || unsafe {
            let style_props = style_props.get();
            let font_props = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let native_appearance = uses_native_appearance(
                style_appearance(style_props.as_ref(), appearance.get()),
                font_props.text_color,
            );
            let owner_draw = !native_appearance && font_props.text_color.is_some();
            if native_appearance {
                SetWindowTheme(hwnd, PCWSTR::null(), PCWSTR::null()).unwrap();
            } else {
                SetWindowTheme(hwnd, w!(""), w!("")).unwrap();
            }
            set_owner_draw(hwnd, owner_draw);
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(
                    resolved_font(&font_props, scale_factor.get()).0 as _,
                )),
                Some(LPARAM(1)), // redraw
            );
            app_state.set_control_text_color(
                hwnd,
                owner_draw.then_some(font_props.text_color).flatten(),
            );
            InvalidateRect(Some(hwnd), None, true).unwrap();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.title,
            props.font.font_family,
            props.font.font_size,
            props.font.font_weight,
            props.font.font_style,
            props.font.text_color,
            props.container.padding(),
            props.view.width,
            props.view.height,
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let font_props = resolve_font_props(
                style_props.as_ref(),
                font_family.get(),
                font_size.get(),
                font_weight.get(),
                font_style.get(),
                text_color.get(),
            );
            let padding = logical_padding(
                style_padding_with_default(style_props.as_ref(), padding.get(), Dimension::Auto),
                scale_factor,
            );

            let hds = unsafe { GetDC(Some(hwnd)) };
            let title = title.get();
            let string = HSTRING::from(&title);
            unsafe {
                SetWindowTextW(hwnd, &string).unwrap();
            }

            let mesure_string = HSTRING::from(if title.is_empty() { "t" } else { &title });
            let mut size: SIZE = SIZE::default();
            unsafe {
                let font = resolved_font(&font_props, scale_factor);
                SelectObject(hds, font.into());
                GetTextExtentPoint32W(hds, &mesure_string, &mut size).unwrap();
                DeleteObject(font.into()).unwrap();
            }

            let width = style_dimension(
                style_props.as_ref(),
                width.get(),
                Dimension::Auto,
                |style| style.width,
            );
            let height = style_dimension(
                style_props.as_ref(),
                height.get(),
                Dimension::Auto,
                |style| style.height,
            );

            let width = match width {
                Dimension::Auto => LogicalUnit::new(
                    PhysicalUnit::new(size.cx).to_logical::<f32>(scale_factor).0
                        + padding.left
                        + padding.right,
                ),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor),
            };
            let height = match height {
                Dimension::Auto => LogicalUnit::new(
                    PhysicalUnit::new(size.cy).to_logical::<f32>(scale_factor).0
                        + padding.top
                        + padding.bottom,
                ),
                Dimension::Length(length) => length.to_logical::<f32>(scale_factor).into(),
            };

            tree_context.update_style(node_id, |prev| Style {
                size: Size {
                    width: taffy::Dimension::from_length(width),
                    height: taffy::Dimension::from_length(height),
                },
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.left,
            props.view.top
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();
            let left =
                style_dimension(style_props.as_ref(), left.get(), Dimension::Auto, |style| {
                    style.left
                });
            let top = style_dimension(style_props.as_ref(), top.get(), Dimension::Auto, |style| {
                style.top
            });
            tree_context.update_style(node_id, |prev| Style {
                inset: inset_to_taffy(left, top, scale_factor),
                ..prev
            });
            tree_context.refresh();
        }
    );

    scoped_effect!(
        [
            window_context.scale_factor,
            tree_context,
            style_props,
            props.view.margin()
        ] || {
            let scale_factor = scale_factor.get();
            let style_props = style_props.get();

            tree_context.update_style(node_id, |prev| Style {
                margin: margin_to_taffy(
                    style_margin(style_props.as_ref(), margin.get()),
                    scale_factor,
                ),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [tree_context, style_props, props.view.align_self] || {
            let style_props = style_props.get();
            tree_context.update_style(node_id, |prev| Style {
                align_self: style_align_self(style_props.as_ref(), align_self.get()).to_taffy(),
                ..prev
            });

            tree_context.refresh();
        }
    );

    scoped_effect!(
        [window_context.scale_factor, tree_context] || {
            if let Some(layout) = tree_context.layout(node_id) {
                let scale_factor = scale_factor.get();
                let point = LogicalPosition::new(layout.location.x, layout.location.y)
                    .to_physical(scale_factor);
                let size = LogicalSize::new(layout.size.width, layout.size.height)
                    .to_physical(scale_factor);

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
                }
            }
        }
    );
}

fn uses_native_appearance(
    appearance: Appearance,
    text_color: Option<nestix_native_core::Color>,
) -> bool {
    match appearance {
        Appearance::Native => true,
        Appearance::None => false,
        Appearance::Auto => text_color.is_none(),
    }
}

fn logical_padding(padding: Rect<Dimension>, scale_factor: f64) -> Rect<f32> {
    fn logical(dimension: Dimension, scale_factor: f64, default: f32) -> f32 {
        match dimension {
            Dimension::Auto => default,
            Dimension::Length(value) => value.to_logical::<f32>(scale_factor).0,
        }
    }

    Rect {
        top: logical(padding.top, scale_factor, DEFAULT_PADDING_Y),
        bottom: logical(padding.bottom, scale_factor, DEFAULT_PADDING_Y),
        left: logical(padding.left, scale_factor, DEFAULT_PADDING_X),
        right: logical(padding.right, scale_factor, DEFAULT_PADDING_X),
    }
}

unsafe fn set_owner_draw(hwnd: windows::Win32::Foundation::HWND, owner_draw: bool) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let button_type = if owner_draw {
            BS_OWNERDRAW
        } else {
            BS_PUSHBUTTON
        } as isize;
        let next = (style & !(BS_TYPEMASK as isize)) | button_type;
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            )
            .unwrap();
        }
    }
}

unsafe fn draw_button(
    item: &DRAWITEMSTRUCT,
    title: &str,
    text_color: Option<nestix_native_core::Color>,
) {
    unsafe {
        let mut rect = item.rcItem;
        FillRect(item.hDC, &rect, GetSysColorBrush(COLOR_BTNFACE));

        let mut frame_state = DFCS_BUTTONPUSH;
        if item.itemState.0 & ODS_SELECTED.0 != 0 {
            frame_state |= DFCS_PUSHED;
        }
        if item.itemState.0 & ODS_DISABLED.0 != 0 {
            frame_state |= DFCS_INACTIVE;
        }
        let _ = DrawFrameControl(item.hDC, &mut rect, DFC_BUTTON, frame_state);

        if item.itemState.0 & ODS_SELECTED.0 != 0 {
            let _ = OffsetRect(&mut rect, 1, 1);
        }
        SetBkMode(item.hDC, TRANSPARENT);
        let color = if item.itemState.0 & ODS_DISABLED.0 != 0 {
            windows::Win32::Foundation::COLORREF(GetSysColor(COLOR_GRAYTEXT))
        } else {
            text_color
                .map(crate::font::colorref)
                .unwrap_or_else(|| windows::Win32::Foundation::COLORREF(GetSysColor(COLOR_BTNTEXT)))
        };
        SetTextColor(item.hDC, color);

        let font = SendMessageW(item.hwndItem, WM_GETFONT, None, None);
        let original_font =
            (font.0 != 0).then(|| SelectObject(item.hDC, HFONT(font.0 as _).into()));
        let mut text: Vec<u16> = title.encode_utf16().collect();
        DrawTextW(
            item.hDC,
            &mut text,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        if let Some(original_font) = original_font {
            SelectObject(item.hDC, original_font);
        }

        if item.itemState.0 & ODS_FOCUS.0 != 0 {
            let _ = InflateRect(&mut rect, -3, -3);
            let _ = DrawFocusRect(item.hDC, &rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nestix_native_core::{Color, dpi::PhysicalUnit};

    #[test]
    fn appearance_controls_native_theme() {
        assert!(uses_native_appearance(Appearance::Native, Some(Color::RED)));
        assert!(!uses_native_appearance(Appearance::None, None));
        assert!(uses_native_appearance(Appearance::Auto, None));
        assert!(!uses_native_appearance(Appearance::Auto, Some(Color::RED)));
    }

    #[test]
    fn padding_uses_defaults_and_scales_physical_values() {
        let padding = logical_padding(
            Rect {
                top: Dimension::Auto,
                bottom: Dimension::from(4),
                left: Dimension::Length(PhysicalUnit::new(8).into()),
                right: Dimension::Auto,
            },
            2.0,
        );

        assert_eq!(padding.top, DEFAULT_PADDING_Y);
        assert_eq!(padding.bottom, 4.0);
        assert_eq!(padding.left, 4.0);
        assert_eq!(padding.right, DEFAULT_PADDING_X);
    }
}
