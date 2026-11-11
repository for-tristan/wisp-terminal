use serde::{Deserialize, Serialize};

use crate::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Beam,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        let value = value.trim();
        let hex = value
            .strip_prefix('#')
            .ok_or_else(|| ConfigError::InvalidColor {
                value: value.to_string(),
            })?;
        let parsed = match hex.len() {
            3 => {
                let bytes = hex
                    .chars()
                    .map(|c| u8::from_str_radix(&c.to_string(), 16).map(|v| v * 17))
                    .collect::<Result<Vec<u8>, _>>()
                    .map_err(|_| ConfigError::InvalidColor {
                        value: value.to_string(),
                    })?;
                [bytes[0], bytes[1], bytes[2]]
            }
            6 => {
                let bytes = (0..3)
                    .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
                    .collect::<Result<Vec<u8>, _>>()
                    .map_err(|_| ConfigError::InvalidColor {
                        value: value.to_string(),
                    })?;
                [bytes[0], bytes[1], bytes[2]]
            }
            _ => {
                return Err(ConfigError::InvalidColor {
                    value: value.to_string(),
                })
            }
        };
        Ok(Self {
            r: parsed[0],
            g: parsed[1],
            b: parsed[2],
        })
    }

    pub const fn to_tuple(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    pub fn to_linear_f32(self) -> [f32; 3] {
        const GAMMA: f32 = 2.2;
        [
            (self.r as f32 / 255.0).powf(GAMMA),
            (self.g as f32 / 255.0).powf(GAMMA),
            (self.b as f32 / 255.0).powf(GAMMA),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub cursor_text: String,
    pub selection_background: String,
    pub selection_foreground: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl Default for Palette {
    fn default() -> Self {
        builtin("wisp").expect("built-in wisp palette").palette
    }
}

impl Palette {
    pub fn color(&self, key: &str) -> Result<Color, ConfigError> {
        let value = match key {
            "background" => &self.background,
            "foreground" => &self.foreground,
            "cursor" => &self.cursor,
            "cursor_text" => &self.cursor_text,
            "selection_background" => &self.selection_background,
            "selection_foreground" => &self.selection_foreground,
            "black" => &self.black,
            "red" => &self.red,
            "green" => &self.green,
            "yellow" => &self.yellow,
            "blue" => &self.blue,
            "magenta" => &self.magenta,
            "cyan" => &self.cyan,
            "white" => &self.white,
            "bright_black" => &self.bright_black,
            "bright_red" => &self.bright_red,
            "bright_green" => &self.bright_green,
            "bright_yellow" => &self.bright_yellow,
            "bright_blue" => &self.bright_blue,
            "bright_magenta" => &self.bright_magenta,
            "bright_cyan" => &self.bright_cyan,
            "bright_white" => &self.bright_white,
            _ => return Err(ConfigError::InvalidColor { value: key.to_string() }),
        };
        Color::parse(value)
    }

    pub fn ansi(&self) -> Result<[Color; 16], ConfigError> {
        Ok([
            self.color("black")?,
            self.color("red")?,
            self.color("green")?,
            self.color("yellow")?,
            self.color("blue")?,
            self.color("magenta")?,
            self.color("cyan")?,
            self.color("white")?,
            self.color("bright_black")?,
            self.color("bright_red")?,
            self.color("bright_green")?,
            self.color("bright_yellow")?,
            self.color("bright_blue")?,
            self.color("bright_magenta")?,
            self.color("bright_cyan")?,
            self.color("bright_white")?,
        ])
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
}

pub fn builtin(name: &str) -> Option<Theme> {
    match name {
        "wisp" => Some(Theme {
            name: "wisp".to_string(),
            palette: Palette {
                background: "#1A222C".to_string(),
                foreground: "#C4C9D2".to_string(),
                cursor: "#E1E3E9".to_string(),
                cursor_text: "#1A222C".to_string(),
                selection_background: "#3D5A73".to_string(),
                selection_foreground: "#E6EBF0".to_string(),
                black: "#22272E".to_string(),
                red: "#C56B6B".to_string(),
                green: "#7FAF7F".to_string(),
                yellow: "#D0B16E".to_string(),
                blue: "#6FA3CB".to_string(),
                magenta: "#A98BC0".to_string(),
                cyan: "#6FB0A8".to_string(),
                white: "#C4C9D2".to_string(),
                bright_black: "#5A6875".to_string(),
                bright_red: "#E08080".to_string(),
                bright_green: "#94C694".to_string(),
                bright_yellow: "#E5C684".to_string(),
                bright_blue: "#8FB9DB".to_string(),
                bright_magenta: "#C0A2D6".to_string(),
                bright_cyan: "#8BC6BC".to_string(),
                bright_white: "#E1E3E9".to_string(),
            },
        }),
        _ => None,
    }
}

pub fn builtin_names() -> Vec<&'static str> {
    vec!["wisp"]
}

pub fn resolve(name: Option<&str>, override_colors: Option<Palette>) -> Theme {
    let mut theme = match name {
        Some(name) => builtin(name).unwrap_or_else(|| {
            log::warn!("unknown theme {name:?}, falling back to built-in \"wisp\"");
            builtin("wisp").expect("built-in wisp theme")
        }),
        None => builtin("wisp").expect("built-in wisp theme"),
    };
    if let Some(colors) = override_colors {
        theme.palette = colors;
    }
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex() {
        let color = Color::parse("#C4C9D2").unwrap();
        assert_eq!(color.to_tuple(), (0xC4, 0xC9, 0xD2));
    }

    #[test]
    fn parses_three_digit_hex() {
        let color = Color::parse("#abc").unwrap();
        assert_eq!(color.to_tuple(), (0xAA, 0xBB, 0xCC));
    }

    #[test]
    fn rejects_missing_hash() {
        assert!(Color::parse("C4C9D2").is_err());
    }

    #[test]
    fn wisp_builtin_resolves() {
        let theme = builtin("wisp").unwrap();
        assert_eq!(theme.palette.color("cursor").unwrap().to_tuple(), (0xE1, 0xE3, 0xE9));
    }
}
