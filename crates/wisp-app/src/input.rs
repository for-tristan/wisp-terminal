use std::time::Instant;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use wisp_core::{Column, Line, Point, SelectionType};

pub enum EncodedInput {
    Bytes(Vec<u8>),
    Copy,
    Paste,
}

pub struct MouseState {
    pub position: Option<PhysicalPosition<f64>>,
    pub left_down: bool,
    pub click_count: u32,
    pub last_click: Option<Instant>,
    pub origin: (f32, f32),
    pub cell_size: (f32, f32),
    pub grid: (usize, usize),
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            position: None,
            left_down: false,
            click_count: 0,
            last_click: None,
            origin: (0.0, 0.0),
            cell_size: (9.0, 18.0),
            grid: (80, 24),
        }
    }

    pub fn viewport_point(&self, position: PhysicalPosition<f64>) -> Option<Point> {
        let (ox, oy) = self.origin;
        let (cw, ch) = self.cell_size;
        let (cols, rows) = self.grid;
        if cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let column = ((position.x - ox as f64) / cw as f64).floor();
        let row = ((position.y - oy as f64) / ch as f64).floor();
        if column < 0.0 || row < 0.0 {
            return None;
        }
        let column = (column as usize).min(cols.saturating_sub(1));
        let row = (row as usize).min(rows.saturating_sub(1));
        Some(Point::new(Line(row as i32), Column(column)))
    }
}

pub fn encode_key(event: &KeyEvent, modifiers: ModifiersState) -> Option<EncodedInput> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.control_key();
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();

    if ctrl && shift {
        match &event.logical_key {
            Key::Character(text) if text.as_str() == "C" || text.as_str() == "c" => {
                return Some(EncodedInput::Copy);
            }
            Key::Character(text) if text.as_str() == "V" || text.as_str() == "v" => {
                return Some(EncodedInput::Paste);
            }
            _ => {}
        }
    }

    if ctrl && !alt {
        if let Key::Character(text) = &event.logical_key {
            if let Some(encoded) = encode_control_char(text.as_str(), shift) {
                return Some(EncodedInput::Bytes(vec![encoded]));
            }
        }
    }

    let mut sequence: Vec<u8> = Vec::new();

    match &event.logical_key {
        Key::Named(named) => {
            let base: &[u8] = match named {
                NamedKey::Enter => b"\r",
                NamedKey::Backspace => b"\x7f",
                NamedKey::Escape => b"\x1b",
                NamedKey::Tab => {
                    if shift {
                        b"\x1b[Z".as_slice()
                    } else {
                        b"\t".as_slice()
                    }
                }
                NamedKey::ArrowLeft => b"\x1b[D",
                NamedKey::ArrowRight => b"\x1b[C",
                NamedKey::ArrowUp => b"\x1b[A",
                NamedKey::ArrowDown => b"\x1b[B",
                NamedKey::Home => b"\x1b[H",
                NamedKey::End => b"\x1b[F",
                NamedKey::PageUp => b"\x1b[5~",
                NamedKey::PageDown => b"\x1b[6~",
                NamedKey::Delete => b"\x1b[3~",
                NamedKey::Insert => b"\x1b[2~",
                NamedKey::Space => b" ",
                _ => return None,
            };
            sequence.extend_from_slice(base);
        }
        Key::Character(text) => {
            let bytes = text.as_bytes();
            if alt {
                sequence.push(0x1b);
            }
            sequence.extend_from_slice(bytes);
        }
        _ => return None,
    }

    if sequence.is_empty() {
        None
    } else {
        Some(EncodedInput::Bytes(sequence))
    }
}

pub fn selection_type_for_click(click_count: u32) -> Option<SelectionType> {
    match click_count {
        1 => Some(SelectionType::Simple),
        2 => Some(SelectionType::Semantic),
        3 => Some(SelectionType::Lines),
        _ => None,
    }
}

pub fn encode_control_char(text: &str, shift: bool) -> Option<u8> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let ch = ch.to_ascii_lowercase();
    let code = match ch {
        '@' => 0x00,
        'a'..='z' => (ch as u8) - b'a' + 1,
        '[' => 0x1b,
        '\\' => 0x1c,
        ']' => 0x1d,
        '^' => 0x1e,
        '_' => 0x1f,
        ' ' => 0x00,
        '?' if shift => 0x7f,
        _ => return None,
    };
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_chars() {
        assert_eq!(encode_control_char("c", false), Some(0x03));
        assert_eq!(encode_control_char("C", false), Some(0x03));
        assert_eq!(encode_control_char("a", false), Some(0x01));
        assert_eq!(encode_control_char("@", false), Some(0x00));
        assert_eq!(encode_control_char("[", false), Some(0x1b));
        assert_eq!(encode_control_char("1", false), None);
    }

    #[test]
    fn click_counts_map_to_selection_types() {
        assert!(matches!(
            selection_type_for_click(1),
            Some(SelectionType::Simple)
        ));
        assert!(matches!(
            selection_type_for_click(2),
            Some(SelectionType::Semantic)
        ));
        assert!(matches!(
            selection_type_for_click(3),
            Some(SelectionType::Lines)
        ));
        assert!(selection_type_for_click(4).is_none());
    }
}
