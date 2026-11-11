use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod theme;

pub use theme::{Color, CursorShape, Palette, Theme};

const CONFIG_DIR_ENV_FALLBACK: &str = "wisp";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid color {value:?}: expected #RGB or #RRGGBB")]
    InvalidColor { value: String },
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub font: FontConfig,
    pub terminal: TerminalConfig,
    pub cursor: CursorConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub padding: Padding,
    pub opacity: f32,
    pub title: String,
    pub min_width: u32,
    pub min_height: u32,
    pub decorations: bool,
    pub maximized: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Padding {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    pub family: String,
    pub size_pt: f32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub shell: Option<String>,
    pub scrollback: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CursorConfig {
    pub style: CursorStyle,
    pub blink: bool,
    pub blink_interval_ms: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: Option<String>,
    pub colors: Option<Palette>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            padding: Padding::default(),
            opacity: 0.85,
            title: "wisp".to_string(),
            min_width: 320,
            min_height: 180,
            decorations: true,
            maximized: false,
        }
    }
}

impl Default for Padding {
    fn default() -> Self {
        Self { x: 24, y: 20 }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "IBM Plex Mono".to_string(),
            size_pt: 14.0,
            line_height: 1.3,
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: None,
            scrollback: 10_000,
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: CursorStyle::Block,
            blink: true,
            blink_interval_ms: 500,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Config = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: Option<&Path>) -> Result<Self, ConfigError> {
        match path {
            Some(path) => Self::load(path),
            None => match Self::default_path() {
                Some(path) if path.exists() => Self::load(&path),
                _ => Ok(Self::default()),
            },
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(CONFIG_DIR_ENV_FALLBACK).join("wisp.toml"))
    }

    pub fn theme(&self) -> Theme {
        theme::resolve(self.theme.name.as_deref(), self.theme.colors.clone())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if !(6.0..=72.0).contains(&self.font.size_pt) {
            return Err(ConfigError::InvalidValue {
                field: "font.size_pt",
                reason: format!("must be within 6.0..=72.0, got {}", self.font.size_pt),
            });
        }
        if !(1.0..=3.0).contains(&self.font.line_height) {
            return Err(ConfigError::InvalidValue {
                field: "font.line_height",
                reason: format!("must be within 1.0..=3.0, got {}", self.font.line_height),
            });
        }
        if !(0.0..=1.0).contains(&self.window.opacity) {
            return Err(ConfigError::InvalidValue {
                field: "window.opacity",
                reason: format!("must be within 0.0..=1.0, got {}", self.window.opacity),
            });
        }
        if self.terminal.scrollback > 1_000_000 {
            return Err(ConfigError::InvalidValue {
                field: "terminal.scrollback",
                reason: "must not exceed 1_000_000".to_string(),
            });
        }
        if self.theme.colors.is_none() {
            if let Some(name) = &self.theme.name {
                if theme::builtin(name).is_none() {
                    return Err(ConfigError::InvalidValue {
                        field: "theme.name",
                        reason: format!("unknown theme {name:?}"),
                    });
                }
            }
        }
        Ok(())
    }
}

impl CursorStyle {
    pub fn default_shape(self) -> crate::theme::CursorShape {
        match self {
            CursorStyle::Block => crate::theme::CursorShape::Block,
            CursorStyle::Beam => crate::theme::CursorShape::Beam,
            CursorStyle::Underline => crate::theme::CursorShape::Underline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_config_fills_defaults() {
        let config: Config =
            toml::from_str("[font]\nsize_pt = 18.0\n[window]\npadding.x = 40\n").unwrap();
        assert_eq!(config.font.size_pt, 18.0);
        assert_eq!(config.font.family, "IBM Plex Mono");
        assert_eq!(config.window.padding.x, 40);
        assert_eq!(config.window.padding.y, 20);
    }

    #[test]
    fn unknown_theme_is_rejected() {
        let config: Config = toml::from_str("[theme]\nname = \"nope\"\n").unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_font_size_is_rejected() {
        let config: Config = toml::from_str("[font]\nsize_pt = 200.0\n").unwrap();
        assert!(config.validate().is_err());
    }
}
