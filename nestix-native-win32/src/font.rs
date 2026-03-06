use windows::{
    Win32::Graphics::Gdi::{
        CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
        FF_DONTCARE, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
    },
    core::w,
};

pub fn ui_font(font_size: f64, scale_factor: f64) -> HFONT {
    unsafe {
        CreateFontW(
            -(font_size * scale_factor) as i32,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0).into(),
            w!("Segoe UI"),
        )
    }
}
