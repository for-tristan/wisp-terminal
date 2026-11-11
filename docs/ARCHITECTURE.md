# Wisp architecture

Wisp is a GPU-accelerated terminal emulator built as a four-crate Cargo workspace.
The crates form a strict one-way dependency chain:

```
wisp-config -> wisp-core -> wisp-render -> wisp-app
```

## Crate responsibilities

### wisp-config

Pure data. `serde` structs for every `[section]` of `wisp.toml`, theme resolution,
hex color parsing and value validation. No windowing, GPU or I/O dependencies beyond
`dirs` for the default config path.

### wisp-core

The emulation heart, with no GUI dependencies:

- `pty::PtySession` - spawns the shell through `portable-pty`, pumps PTY output on a
  dedicated reader thread into an mpsc channel (with an optional wake callback for
  the UI event loop), exposes write/resize/kill, and watches for child exit.
- `terminal::TerminalModel` - wraps `alacritty_terminal`'s `Term` grid: scrollback
  history, ANSI/VT parsing through the `vte` processor, selection state, bracketed
  paste encoding, viewport-to-grid coordinate conversion.
- `emulator::Emulator` - owns PTY + model, drains bytes and terminal events each
  frame (`pump`), answers OSC requests (colors, text-area size, clipboard store/
  load), resolves 256-color sequences from the active palette, and reports app-level
  events (title change, bell, child exit).
- `snapshot::Snapshot` - a plain-data viewport render: per-cell char/fg/bg/flags with
  theme colors resolved, inverse and dim applied, selection highlight merged, plus
  cursor position/shape. The GPU layer only ever sees this type.

### wisp-render

Everything visual:

- `gpu::GpuContext` - wgpu instance/adapter/device/queue + surface lifecycle with
  swapchain reconfiguration and Outdated/Lost recovery.
- `font::FontStack` - system font discovery via `font-kit` with a bundled IBM Plex
  Mono fallback (SIL OFL), four style variants, cell metrics (advance width, line
  height, baseline offset).
- `atlas::GlyphAtlas` - 2048x2048 R8 texture backed by rusttype's gpu_cache; glyphs
  rasterize once, then draw as UV quads; on overflow the atlas invalidates itself.
- `pipeline::CellPipeline` - one instanced WGSL pipeline, two passes per frame:
  solid quads (cell backgrounds, cursor, underline/strikeout, scrollbar) then glyph
  quads (alpha-blended atlas samples). Instances are a single GPU buffer with two
  draw ranges.
- `renderer::Renderer` - builds the instance list from a `Snapshot` and submits the
  frame.

## Frame loop

```
PTY bytes -> reader thread -> mpsc -> wake proxy -> UserEvent::PtyData
   -> pump() (feed parser, drain events) -> needs_redraw = true
   -> RedrawRequested
        -> pump() again (drain stragglers)
        -> emulator.snapshot() -> renderer.render(snapshot)
```

Redraws only happen when the emulator reports damage or the cursor blink timer
fires, so an idle terminal consumes no GPU cycles while staying fully responsive.

## Design decisions

- The renderer never imports emulator internals; the snapshot boundary keeps the
  GPU crate swappable and testable.
- The PTY reader thread never blocks the UI thread; wakeups go through a winit
  `EventLoopProxy`.
- Selections live inside the emulator grid so semantics (word/line expansion,
  scrolled history) stay correct; the app only feeds it points.
- All configuration values are validated at load time with explicit errors instead
  of silent clamping.
