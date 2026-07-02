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

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "white" => Ok(Color::WHITE),
            "black" => Ok(Color::BLACK),
            "transparent" => Ok(Color::TRANSPARENT),
            "red" => Ok(Color::RED),
            "green" => Ok(Color::GREEN),
            "blue" => Ok(Color::BLUE),
            _ => {
                let rgb_color = RGBColor::parse(value)?;
                Ok(Self::RGB(rgb_color))
            }
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
        Self {
            red: r,
            green: g,
            blue: b,
            alpha: 255,
        }
    }

    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());

        if hex.len() != 6 && hex.len() != 8 {
            return Err("Hex colour must be 6 or 8 characters long".to_string());
        }

        let parse_pair = |i: usize| -> Result<u8, String> {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| format!("Invalid hex value: {}", &hex[i..i + 2]))
        };

        Ok(Self {
            red: parse_pair(0)?,
            green: parse_pair(2)?,
            blue: parse_pair(4)?,
            alpha: if hex.len() == 8 { parse_pair(6)? } else { 255 },
        })
    }
}

impl From<RGBColor> for Color {
    fn from(value: RGBColor) -> Self {
        Self::RGB(value)
    }
}
