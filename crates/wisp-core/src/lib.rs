pub mod emulator;
pub mod pty;
pub mod snapshot;
pub mod terminal;

pub const CRATE_NAME: &str = "wisp-core";

pub use alacritty_terminal::index::{Column, Line, Point, Side};
pub use alacritty_terminal::selection::SelectionType;
