use winit::event::ElementState;
use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

pub fn encode_key(event: &KeyEvent, modifiers: ModifiersState) -> Option<Vec<u8>> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.control_key();
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();

    if ctrl && !alt {
        if let Key::Character(text) = &event.logical_key {
            if let Some(encoded) = encode_control_char(text.as_str(), shift) {
                return Some(vec![encoded]);
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
        Some(sequence)
    }
}

fn encode_control_char(text: &str, shift: bool) -> Option<u8> {
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
