use nestix::props;

use crate::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
    Numeric(u16),
}

impl FontWeight {
    pub fn value(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
            Self::Numeric(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct FontProps {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub text_color: Option<Color>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedFontProps {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub text_color: Option<Color>,
}
