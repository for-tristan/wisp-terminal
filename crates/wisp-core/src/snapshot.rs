use alacritty_terminal::term::cell::Flags as TermFlags;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use wisp_config::{Color as ConfigColor, Palette};

pub const FLAG_BOLD: u16 = 1 << 0;
pub const FLAG_ITALIC: u16 = 1 << 1;
pub const FLAG_DIM: u16 = 1 << 2;
pub const FLAG_UNDERLINE: u16 = 1 << 3;
pub const FLAG_DOUBLE_UNDERLINE: u16 = 1 << 4;
pub const FLAG_STRIKEOUT: u16 = 1 << 5;
pub const FLAG_WIDE: u16 = 1 << 6;
pub const FLAG_HIDDEN: u16 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    Block,
    Beam,
    Underline,
    HollowBlock,
}

#[derive(Debug, Clone)]
pub struct SnapshotCell {
    pub row: u16,
    pub column: u16,
    pub character: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub flags: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotCursor {
    pub row: u16,
    pub column: u16,
    pub kind: CursorKind,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub columns: usize,
    pub screen_lines: usize,
    pub cells: Vec<SnapshotCell>,
    pub cursor: Option<SnapshotCursor>,
    pub display_offset: usize,
}

pub fn indexed_color(index: usize, ansi: &[[u8; 3]; 16]) -> [u8; 3] {
    if index < 16 {
        return ansi[index];
    }
    if (16..232).contains(&index) {
        let index = index - 16;
        let r = (index / 36) as u8;
        let g = ((index / 6) % 6) as u8;
        let b = (index % 6) as u8;
        let channel = |v: u8| if v == 0 { 0 } else { (v - 1) * 40 + 95 };
        return [channel(r), channel(g), channel(b)];
    }
    let value = ((index - 232) as u32 * 10 + 8) as u8;
    [value, value, value]
}

fn dim(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|value| ((value as f32) * 0.6) as u8)
}

fn fallback_ansi_rgb() -> [[u8; 3]; 16] {
    crate::emulator::fallback_ansi().map(|color| {
        let (r, g, b) = color.to_tuple();
        [r, g, b]
    })
}

fn resolve_color(palette: &Palette, ansi: &[[u8; 3]; 16], color: Color) -> [u8; 3] {
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Indexed(index) => indexed_color(index as usize, ansi),
        Color::Named(named) => {
            let pick = |key: &str| -> [u8; 3] {
                palette
                    .color(key)
                    .map(|color: ConfigColor| color.to_tuple())
                    .map(|(r, g, b)| [r, g, b])
                    .unwrap_or([255, 255, 255])
            };
            match named {
                NamedColor::Foreground | NamedColor::BrightForeground => pick("foreground"),
                NamedColor::Background => pick("background"),
                NamedColor::Cursor => pick("cursor"),
                NamedColor::Black => ansi[0],
                NamedColor::Red => ansi[1],
                NamedColor::Green => ansi[2],
                NamedColor::Yellow => ansi[3],
                NamedColor::Blue => ansi[4],
                NamedColor::Magenta => ansi[5],
                NamedColor::Cyan => ansi[6],
                NamedColor::White => ansi[7],
                NamedColor::BrightBlack => ansi[8],
                NamedColor::BrightRed => ansi[9],
                NamedColor::BrightGreen => ansi[10],
                NamedColor::BrightYellow => ansi[11],
                NamedColor::BrightBlue => ansi[12],
                NamedColor::BrightMagenta => ansi[13],
                NamedColor::BrightCyan => ansi[14],
                NamedColor::BrightWhite => ansi[15],
                NamedColor::DimBlack => dim(ansi[0]),
                NamedColor::DimRed => dim(ansi[1]),
                NamedColor::DimGreen => dim(ansi[2]),
                NamedColor::DimYellow => dim(ansi[3]),
                NamedColor::DimBlue => dim(ansi[4]),
                NamedColor::DimMagenta => dim(ansi[5]),
                NamedColor::DimCyan => dim(ansi[6]),
                NamedColor::DimWhite => dim(ansi[7]),
                NamedColor::DimForeground => dim(pick("foreground")),
            }
        }
    }
}

fn map_flags(flags: TermFlags) -> u16 {
    let mut out = 0u16;
    if flags.contains(TermFlags::BOLD) {
        out |= FLAG_BOLD;
    }
    if flags.contains(TermFlags::ITALIC) {
        out |= FLAG_ITALIC;
    }
    if flags.contains(TermFlags::DIM) {
        out |= FLAG_DIM;
    }
    if flags.contains(TermFlags::UNDERLINE) {
        out |= FLAG_UNDERLINE;
    }
    if flags.contains(TermFlags::DOUBLE_UNDERLINE) {
        out |= FLAG_DOUBLE_UNDERLINE;
    }
    if flags.contains(TermFlags::STRIKEOUT) {
        out |= FLAG_STRIKEOUT;
    }
    if flags.contains(TermFlags::WIDE_CHAR) {
        out |= FLAG_WIDE;
    }
    if flags.contains(TermFlags::HIDDEN) {
        out |= FLAG_HIDDEN;
    }
    out
}

fn cursor_kind(shape: CursorShape) -> Option<CursorKind> {
    match shape {
        CursorShape::Block => Some(CursorKind::Block),
        CursorShape::Beam => Some(CursorKind::Beam),
        CursorShape::Underline => Some(CursorKind::Underline),
        CursorShape::HollowBlock => Some(CursorKind::HollowBlock),
        CursorShape::Hidden => None,
    }
}

impl crate::terminal::TerminalModel {
    pub fn snapshot(&self, palette: &Palette) -> Snapshot {
        let content = self.term().renderable_content();
        let columns = self.dims().columns;
        let screen_lines = self.dims().screen_lines;

        let ansi: [[u8; 3]; 16] = match palette.ansi() {
            Ok(colors) => colors.map(|color| {
                let (r, g, b) = color.to_tuple();
                [r, g, b]
            }),
            Err(_) => fallback_ansi_rgb(),
        };

        let selection_background = palette
            .color("selection_background")
            .ok()
            .map(|color| color.to_tuple())
            .map(|(r, g, b)| [r, g, b]);
        let selection_foreground = palette
            .color("selection_foreground")
            .ok()
            .map(|color| color.to_tuple())
            .map(|(r, g, b)| [r, g, b]);

        let mut cells = Vec::with_capacity(columns * screen_lines);
        for (index, cell) in content.display_iter.enumerate() {
            let row = (index / columns) as u16;
            let column = (index % columns) as u16;

            if cell
                .flags
                .intersects(TermFlags::WIDE_CHAR_SPACER | TermFlags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let mut fg = resolve_color(palette, &ansi, cell.fg);
            let mut bg = resolve_color(palette, &ansi, cell.bg);

            if cell.flags.contains(TermFlags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if cell.flags.contains(TermFlags::DIM) {
                fg = dim(fg);
            }

            if let (Some(range), Some(sel_bg), Some(sel_fg)) = (
                &content.selection,
                selection_background,
                selection_foreground,
            ) {
                if range.contains(cell.point) {
                    bg = sel_bg;
                    fg = sel_fg;
                }
            }

            cells.push(SnapshotCell {
                row,
                column,
                character: cell.c,
                fg,
                bg,
                flags: map_flags(cell.flags),
            });
        }

        let cursor = cursor_kind(content.cursor.shape).map(|kind| SnapshotCursor {
            row: (content.cursor.point.line.0 + content.display_offset as i32) as u16,
            column: content.cursor.point.column.0 as u16,
            kind,
        });

        Snapshot {
            columns,
            screen_lines,
            cells,
            cursor,
            display_offset: content.display_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::TerminalModel;
    use wisp_config::Config;

    #[test]
    fn snapshot_covers_viewport() {
        let config = Config::default();
        let mut model = TerminalModel::new(&config, 10, 4);
        model.feed(b"hi");
        let palette = wisp_config::theme::builtin("wisp")
            .expect("wisp theme")
            .palette;
        let snapshot = model.snapshot(&palette);
        assert_eq!(snapshot.columns, 10);
        assert_eq!(snapshot.screen_lines, 4);
        assert_eq!(snapshot.cells.len(), 40);
        let h = &snapshot.cells[0];
        assert_eq!(h.character, 'h');
        assert_eq!(h.row, 0);
        assert_eq!(h.column, 0);
    }

    #[test]
    fn snapshot_cursor_position() {
        let config = Config::default();
        let mut model = TerminalModel::new(&config, 10, 4);
        model.feed(b"abc");
        let palette = wisp_config::theme::builtin("wisp")
            .expect("wisp theme")
            .palette;
        let snapshot = model.snapshot(&palette);
        let cursor = snapshot.cursor.expect("cursor");
        assert_eq!(cursor.column, 3);
        assert_eq!(cursor.row, 0);
    }

    #[test]
    fn indexed_color_cube_and_gray() {
        let ansi = [[0u8; 3]; 16];
        let red = indexed_color(196, &ansi);
        assert_eq!(red, [255, 0, 0]);
        let gray = indexed_color(232, &ansi);
        assert_eq!(gray, [8, 8, 8]);
    }

    #[test]
    fn selection_highlight_uses_palette() {
        use alacritty_terminal::index::{Column, Line, Point, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};
        let to_arr = |tuple: (u8, u8, u8)| [tuple.0, tuple.1, tuple.2];

        let config = Config::default();
        let mut model = TerminalModel::new(&config, 10, 4);
        model.feed(b"selected!");
        let mut selection = Selection::new(
            SelectionType::Simple,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        selection.update(Point::new(Line(0), Column(7)), Side::Right);
        model.set_selection(Some(selection));

        let palette = wisp_config::theme::builtin("wisp")
            .expect("wisp theme")
            .palette;
        let selection_bg = palette
            .color("selection_background")
            .expect("selection bg")
            .to_tuple();
        let selection_fg = palette
            .color("selection_foreground")
            .expect("selection fg")
            .to_tuple();

        let snapshot = model.snapshot(&palette);
        let selected = &snapshot.cells[0];
        assert_eq!(selected.bg, to_arr(selection_bg));
        assert_eq!(selected.fg, to_arr(selection_fg));
        let unselected = &snapshot.cells[9];
        assert_ne!(unselected.bg, to_arr(selection_bg));
    }

    #[test]
    fn inverse_swaps_colors() {
        use alacritty_terminal::index::Column;

        let config = Config::default();
        let mut model = TerminalModel::new(&config, 10, 4);
        model.feed(b"\x1b[7mX\x1b[27m");
        let to_arr = |tuple: (u8, u8, u8)| [tuple.0, tuple.1, tuple.2];
        let palette = wisp_config::theme::builtin("wisp")
            .expect("wisp theme")
            .palette;
        let background = palette
            .color("background")
            .expect("background")
            .to_tuple();
        let foreground = palette
            .color("foreground")
            .expect("foreground")
            .to_tuple();
        let snapshot = model.snapshot(&palette);
        let cell = &snapshot.cells[0];
        assert_eq!(cell.character, 'X');
        assert_eq!(cell.fg, to_arr(background));
        assert_eq!(cell.bg, to_arr(foreground));
    }
}
