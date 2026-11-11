# Changelog

## 0.1.0

Initial development release of Wisp, a GPU-accelerated terminal emulator written in Rust.

### Core

- Cross-platform shell spawning via `portable-pty` (config shell, `$SHELL`, or
  platform default) with a dedicated PTY reader thread, wake-proxy plumbing,
  resize support, exit watching and kill-on-drop.
- Full ANSI/VT emulation through `alacritty_terminal`: grid, scrollback history,
  alternate screen, wide characters, bracketed paste and OSC handling
  (title, bell, colors, clipboard store/load OSC 52, text-area size).
- Viewport snapshots with theme-resolved colors, inverse/dim handling, selection
  highlight and cursor shape, so the GPU layer stays emulator-agnostic.

### Rendering

- wgpu-based instanced renderer: solid quads for backgrounds/cursor/decorations,
  alpha-blended glyph quads fed from a 2048x2048 dynamic glyph atlas.
- Font loading via `font-kit` system discovery with bundled IBM Plex Mono
  (SIL OFL) fallback; four style variants; DPI-independent cell metrics.
- Damage-driven redraws with vsync (Fifo) presentation, cursor blink timer,
  hollow cursor when unfocused, scroll position indicator.

### Interaction

- xterm-style keyboard encoding (control chars, navigation, F-keys, Alt prefixes)
  plus IME commit/preedit support anchored to the cell cursor.
- Mouse selection: drag, double-click semantic, triple-click line, right-click
  extend; clipboard copy/paste via `arboard`, OSC 52 passthrough, bracketed paste.
- Scrollback via wheel or Shift+PageUp/PageDown; alternate screens receive arrow
  keys instead.

### Configuration

- `wisp.toml` with window padding/opacity/decorations, font family/size/line
  height, shell, scrollback, cursor style/blink and five built-in themes
  (`wisp`, `nord`, `gruvbox-dark`, `dracula`, `one-dark`) with inline overrides.
- CLI: `--config`, `--working-dir`, `--print-config-path`, `--verbose`.

### Quality

- 20 unit tests across config/core/input, `cargo fmt` clean,
  `cargo clippy -D warnings` clean, GitHub Actions CI on Linux/macOS/Windows.
