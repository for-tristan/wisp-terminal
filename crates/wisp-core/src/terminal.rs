use std::sync::mpsc::{self, Receiver};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Point, Side};
use alacritty_terminal::selection::Selection;
use alacritty_terminal::term::{Config as TermConfig, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;
use wisp_config::Config;

#[derive(Clone)]
pub struct EventProxy {
    tx: mpsc::Sender<Event>,
}

impl EventProxy {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        Self { tx }
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GridDims {
    pub columns: usize,
    pub screen_lines: usize,
}

impl GridDims {
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        Self {
            columns,
            screen_lines,
        }
    }
}

impl Dimensions for GridDims {
    fn columns(&self) -> usize {
        self.columns
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn total_lines(&self) -> usize {
        self.screen_lines
    }
}

pub struct TerminalModel {
    term: Term<EventProxy>,
    processor: Processor,
    event_rx: Receiver<Event>,
    dims: GridDims,
}

impl TerminalModel {
    pub fn new(config: &Config, columns: usize, screen_lines: usize) -> Self {
        let (tx, event_rx) = mpsc::channel();
        let proxy = EventProxy::new(tx);
        let term_config = TermConfig {
            scrolling_history: config.terminal.scrollback as usize,
            ..Default::default()
        };
        let dims = GridDims::new(columns, screen_lines);
        let term = Term::new(term_config, &dims, proxy);
        Self {
            term,
            processor: Processor::new(),
            event_rx,
            dims,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn resize(&mut self, columns: usize, screen_lines: usize) {
        self.dims = GridDims::new(columns, screen_lines);
        self.term.resize(self.dims);
    }

    pub fn dims(&self) -> GridDims {
        self.dims
    }

    pub fn scroll(&mut self, delta_lines: i32) {
        self.term.scroll_display(Scroll::Delta(delta_lines));
    }

    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.term.selection = selection;
    }

    pub fn update_selection(&mut self, point: Point, side: Side) {
        if let Some(selection) = &mut self.term.selection {
            selection.update(point, side);
        }
    }

    pub fn selection_text_range(&self) -> Option<String> {
        let term = &self.term;
        let range = term.selection.as_ref()?.to_range(term)?;
        Some(term.bounds_to_string(range.start, range.end))
    }

    pub fn selection_text(&self, start: Point, end: Point) -> String {
        self.term.bounds_to_string(start, end)
    }

    pub fn viewport_to_point(&self, viewport_point: Point<usize>) -> Point {
        alacritty_terminal::term::viewport_to_point(self.display_offset(), viewport_point)
    }

    pub fn encode_paste(&self, text: &str) -> Vec<u8> {
        if self.term.mode().contains(TermMode::BRACKETED_PASTE) {
            let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
            let mut bytes = b"\x1b[200~".to_vec();
            bytes.extend_from_slice(normalized.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            bytes
        } else {
            let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
            normalized.into_bytes()
        }
    }

    pub fn term(&self) -> &Term<EventProxy> {
        &self.term
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line};

    #[test]
    fn feed_and_cursor_advance() {
        let config = Config::default();
        let mut model = TerminalModel::new(&config, 80, 24);
        model.feed(b"hello");
        let cursor = &model.term().grid().cursor;
        assert_eq!(cursor.point, Point::new(Line(0), Column(5)));
        assert_eq!(model.term().grid()[Point::new(Line(0), Column(0))].c, 'h');
    }

    #[test]
    fn newline_moves_cursor_row() {
        let config = Config::default();
        let mut model = TerminalModel::new(&config, 80, 24);
        model.feed(b"one\r\ntwo");
        let cursor = &model.term().grid().cursor;
        assert_eq!(cursor.point.line, Line(1));
    }

    #[test]
    fn encode_paste_plain() {
        let config = Config::default();
        let model = TerminalModel::new(&config, 80, 24);
        assert_eq!(model.encode_paste("a\nb"), b"a\rb".to_vec());
    }
}
