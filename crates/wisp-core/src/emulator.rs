use std::sync::Arc;

use alacritty_terminal::event::{Event, WindowSize};
use alacritty_terminal::index::Point;
use alacritty_terminal::selection::Selection;
use alacritty_terminal::term::color::Colors as TermColors;
use alacritty_terminal::vte::ansi::{NamedColor, Rgb};
use anyhow::{anyhow, Context, Result};
use wisp_config::{Config, Palette};

use crate::pty::{PtyConfig, PtySession};
use crate::snapshot::indexed_color;
use crate::terminal::TerminalModel;

#[derive(Clone)]
pub enum AppEvent {
    TitleChanged(String),
    Bell,
    ClipboardStore(String),
    ClipboardLoadRequest(Arc<dyn Fn(&str) -> String + Send + Sync>),
    ChildExited(Option<u32>),
}

#[derive(Default)]
pub struct PumpResult {
    pub damaged: bool,
    pub events: Vec<AppEvent>,
}

pub struct Emulator {
    pty: PtySession,
    model: TerminalModel,
    palette: Palette,
    cols: usize,
    rows: usize,
    cell_width: u16,
    cell_height: u16,
}

impl Emulator {
    pub fn new(config: &Config, wake: Option<Box<dyn Fn() + Send>>) -> Result<Self> {
        let (cols, rows) = (80usize, 24usize);
        let mut pty_config = PtyConfig::from_config(config);
        pty_config.wake = wake;
        let pty = PtySession::spawn(&mut pty_config)
            .map_err(|e| anyhow!("failed to start shell session: {e}"))
            .context("spawning pty")?;
        log::info!("started shell {} (pid {:?})", pty.shell(), pty.pid());
        let model = TerminalModel::new(config, cols, rows);
        let palette = config.theme().palette;
        Ok(Self {
            pty,
            model,
            palette,
            cols,
            rows,
            cell_width: 9,
            cell_height: 18,
        })
    }

    pub fn set_cell_size(&mut self, width: u16, height: u16) {
        self.cell_width = width.max(1);
        self.cell_height = height.max(1);
    }

    pub fn cell_size(&self) -> (u16, u16) {
        (self.cell_width, self.cell_height)
    }

    pub fn grid_size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    pub fn shell(&self) -> &str {
        self.pty.shell()
    }

    pub fn pump(&mut self) -> PumpResult {
        let mut result = PumpResult::default();

        while let Some(bytes) = self.pty.try_recv() {
            self.model.feed(&bytes);
            result.damaged = true;
        }

        for event in self.model.take_events() {
            match event {
                Event::PtyWrite(text) => {
                    if self.write(text.as_bytes()).is_err() {
                        log::warn!("failed to answer pty request");
                    }
                }
                Event::Title(title) => {
                    result
                        .events
                        .push(AppEvent::TitleChanged(title));
                }
                Event::ResetTitle => {
                    result
                        .events
                        .push(AppEvent::TitleChanged("wisp".to_string()));
                }
                Event::Bell => result.events.push(AppEvent::Bell),
                Event::ClipboardStore(_, text) => {
                    result.events.push(AppEvent::ClipboardStore(text));
                }
                Event::ClipboardLoad(_, formatter) => {
                    result
                        .events
                        .push(AppEvent::ClipboardLoadRequest(formatter));
                }
                Event::ColorRequest(index, formatter) => {
                    let rgb = self.resolve_color(index);
                    if self.write(formatter(rgb).as_bytes()).is_err() {
                        log::warn!("failed to answer color request {index}");
                    }
                }
                Event::TextAreaSizeRequest(formatter) => {
                    let size = WindowSize {
                        num_cols: self.cols as u16,
                        num_lines: self.rows as u16,
                        cell_width: self.cell_width,
                        cell_height: self.cell_height,
                    };
                    if self.write(formatter(size).as_bytes()).is_err() {
                        log::warn!("failed to answer text area size request");
                    }
                }
                Event::MouseCursorDirty | Event::Wakeup | Event::Exit | Event::ChildExit(_) => {}
                Event::CursorBlinkingChange => {}
            }
        }

        if self.pty.has_exited() {
            let code = self.pty.exit_code().flatten();
            log::info!("shell exited with code {:?}", code);
            result.events.push(AppEvent::ChildExited(code));
        }

        result
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.pty
            .write(bytes)
            .context("writing to pty")
    }

    pub fn input(&mut self, bytes: &[u8]) -> Result<()> {
        self.write(bytes)
    }

    pub fn paste(&mut self, text: &str) -> Result<()> {
        let encoded = self.model.encode_paste(text);
        self.write(&encoded)
    }

    pub fn notify_exit(&mut self, code: Option<u32>) {
        let message = match code {
            Some(0) => "\r\n[process exited]\r\n".to_string(),
            Some(code) => format!("\r\n[process exited with code {code}]\r\n"),
            None => "\r\n[process exited]\r\n".to_string(),
        };
        self.model.feed(message.as_bytes());
    }

    pub fn resize(&mut self, cols: usize, rows: usize) -> Result<()> {
        if cols == 0 || rows == 0 || (cols == self.cols && rows == self.rows) {
            return Ok(());
        }
        self.cols = cols;
        self.rows = rows;
        self.model.resize(cols, rows);
        self.pty
            .resize(cols as u16, rows as u16)
            .map_err(|e| anyhow!("pty resize failed: {e}"))
    }

    pub fn scroll(&mut self, delta_lines: i32) {
        self.model.scroll(delta_lines);
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.model.set_selection(selection);
    }

    pub fn selection_text(&self, start: Point, end: Point) -> String {
        self.model.selection_text(start, end)
    }

    pub fn viewport_to_point(&self, viewport_point: Point<usize>) -> Point {
        self.model.viewport_to_point(viewport_point)
    }

    pub fn display_offset(&self) -> usize {
        self.model.display_offset()
    }

    pub fn model(&self) -> &TerminalModel {
        &self.model
    }

    pub fn theme_palette(&self) -> &Palette {
        &self.palette
    }

    pub fn term_colors(&self) -> TermColors {
        let mut colors = TermColors::default();
        let ansi = self
            .palette
            .ansi()
            .unwrap_or_else(|_| fallback_ansi());
        for (index, color) in ansi.iter().enumerate() {
            let (r, g, b) = color.to_tuple();
            colors[index] = Some(Rgb { r, g, b });
        }
        colors[NamedColor::Foreground] = Some(self.palette_rgb("foreground"));
        colors[NamedColor::Background] = Some(self.palette_rgb("background"));
        colors[NamedColor::Cursor] = Some(self.palette_rgb("cursor"));
        colors
    }

    fn palette_rgb(&self, key: &str) -> Rgb {
        let (r, g, b) = self
            .palette
            .color(key)
            .map(|color| color.to_tuple())
            .unwrap_or((0xC4, 0xC9, 0xD2));
        Rgb { r, g, b }
    }

    fn resolve_color(&self, index: usize) -> Rgb {
        let ansi = self
            .palette
            .ansi()
            .unwrap_or_else(|_| fallback_ansi());
        let rgb = indexed_color(
            index,
            &ansi
                .iter()
                .map(|color| color.to_tuple())
                .map(|(r, g, b)| [r, g, b])
                .collect::<Vec<_>>()
                .try_into()
                .expect("16 ansi colors"),
        );
        Rgb {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}

pub fn fallback_ansi() -> [wisp_config::Color; 16] {
    let hex = [
        "#22272E", "#C56B6B", "#7FAF7F", "#D0B16E", "#6FA3CB", "#A98BC0", "#6FB0A8", "#C4C9D2",
        "#5A6875", "#E08080", "#94C694", "#E5C684", "#8FB9DB", "#C0A2D6", "#8BC6BC", "#E1E3E9",
    ];
    let mut colors = Vec::with_capacity(16);
    for value in hex {
        colors.push(wisp_config::Color::parse(value).expect("valid fallback color"));
    }
    colors.try_into().expect("16 colors")
}
