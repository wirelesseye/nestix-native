#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    RGB(RGBColor),
}

impl Color {
    pub const WHITE: Color = Self::RGB(RGBColor::from_rgb(255, 255, 255));
    pub const BLACK: Color = Self::RGB(RGBColor::from_rgb(0, 0, 0));
    pub const TRANSPARENT: Color = Self::RGB(RGBColor::from_rgba(0, 0, 0, 0));
    pub const RED: Color = Self::RGB(RGBColor::from_rgb(255, 0, 0));
    pub const GREEN: Color = Self::RGB(RGBColor::from_rgb(0, 255, 0));
    pub const BLUE: Color = Self::RGB(RGBColor::from_rgb(0, 0, 255));

    pub fn into_rgb(self) -> RGBColor {
        match self {
            Color::RGB(rgb) => rgb,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RGBColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl RGBColor {
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { red: r, green: g, blue: b, alpha: 255 }
    }

    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { red: r, green: g, blue: b, alpha: a }
    }
}

impl From<RGBColor> for Color {
    fn from(value: RGBColor) -> Self {
        Self::RGB(value)
    }
}
