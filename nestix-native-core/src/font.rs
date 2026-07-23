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

/// Optional font properties shared by text-bearing controls.
#[props(debug, default)]
#[derive(Debug, Clone)]
pub struct FontProps {
    /// Preferred font family name.
    pub font_family: Option<String>,
    /// Font size in logical points.
    pub font_size: Option<f64>,
    /// Font weight.
    pub font_weight: Option<FontWeight>,
    /// Font style.
    pub font_style: Option<FontStyle>,
    /// Foreground color of text.
    pub text_color: Option<Color>,
}

/// Concrete font properties after style inheritance and defaults are resolved.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedFontProps {
    /// Resolved font family name.
    pub font_family: Option<String>,
    /// Resolved font size in logical points.
    pub font_size: Option<f64>,
    /// Resolved font weight.
    pub font_weight: Option<FontWeight>,
    /// Resolved font style.
    pub font_style: Option<FontStyle>,
    /// Resolved foreground color.
    pub text_color: Option<Color>,
}
