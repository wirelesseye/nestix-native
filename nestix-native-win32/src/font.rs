use nestix_native_core::{Color, FontStyle, FontWeight, ResolvedFontProps};
use windows::{
    Win32::Foundation::COLORREF,
    Win32::Graphics::Gdi::{
        CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
        FF_DONTCARE, HFONT, OUT_DEFAULT_PRECIS,
    },
    core::HSTRING,
};

pub fn resolved_font(props: &ResolvedFontProps, scale_factor: f64) -> HFONT {
    let family = HSTRING::from(props.font_family.as_deref().unwrap_or("Segoe UI"));
    unsafe {
        CreateFontW(
            -(props.font_size.unwrap_or(12.0) * scale_factor) as i32,
            0,
            0,
            0,
            props.font_weight.unwrap_or(FontWeight::Normal).value() as i32,
            u32::from(props.font_style == Some(FontStyle::Italic)),
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0).into(),
            &family,
        )
    }
}

pub fn ui_font(font_size: f64, scale_factor: f64) -> HFONT {
    resolved_font(
        &ResolvedFontProps {
            font_size: Some(font_size),
            ..Default::default()
        },
        scale_factor,
    )
}

pub fn colorref(color: Color) -> COLORREF {
    let color = color.into_rgb();
    COLORREF(color.red as u32 | (color.green as u32) << 8 | (color.blue as u32) << 16)
}
