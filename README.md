# Wisp

A high-performance, GPU-accelerated terminal emulator written in Rust.

[![CI](https://github.com/for-tristan/wisp-terminal/actions/workflows/ci.yml/badge.svg)](https://github.com/for-tristan/wisp-terminal/actions/workflows/ci.yml)

## Features

- GPU rendering built on wgpu: an instanced cell renderer draws backgrounds,
  cursor and decorations as solid quads and text as alpha-blended glyph quads
  fed from a dynamic 2048x2048 glyph atlas, presented with vsync.
- Full ANSI/VT emulation through `alacritty_terminal`: scrollback history,
  alternate screen, wide characters, bracketed paste, and OSC handling for
  titles, colors and clipboard (OSC 52).
- Sharp text on any display: glyphs are rasterized at the window scale factor
  and snapped to whole physical pixels, so 100% to 200% display scaling stays
  crisp, and moving a window between monitors re-rasterizes automatically.
- Damage-driven redraws: frames are only produced when the grid, selection or
  cursor actually changes.
- Selection and clipboard: drag selection, double-click word, triple-click
  line, right-click extend, plus OSC 52 passthrough.
- IME support: commit events are inserted at the cursor and the IME candidate
  window tracks the cell.
- Configurable from a single `wisp.toml`: window padding, opacity and
  decorations, font family, size and line height, shell, scrollback, cursor
  style and blink, and five built-in themes (`wisp`, `nord`, `gruvbox-dark`,
  `dracula`, `one-dark`) with inline color overrides.
- Cross-platform: Linux, macOS and Windows, tested in CI on all three.

## Requirements

- Rust 1.85 or newer, installed via [rustup](https://rustup.rs).
- A GPU and driver supported by wgpu (Direct3D 12, Metal, or Vulkan).
- Linux: FreeType and fontconfig development packages for `font-kit`
  (for example `libfreetype6-dev` and `libfontconfig1-dev` on Debian/Ubuntu).
- Windows: the MSVC toolchain is recommended. Install
  [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
  with the "Desktop development with C++" workload. The `windows-gnu` target
  also works but needs mingw-w64 binutils (`dlltool`, `as`) on your PATH, for
  example from [MSYS2](https://www.msys2.org/).

## Building and running

```sh
git clone https://github.com/for-tristan/wisp-terminal.git
cd wisp-terminal
cargo build --release
```

The binary is written to `target/release/wisp`. Launch it directly, or use:

```sh
cargo run --release
```

### Command line options

| Option | Description |
| --- | --- |
| `-c, --config FILE` | Path to a `wisp.toml` configuration file |
| `-w, --working-dir DIR` | Directory the shell starts in |
| `--print-config-path` | Print the config path Wisp would load and exit |
| `-v, --verbose` | Enable debug logging |

## Configuration

Wisp loads its configuration from the first existing location:

1. The path passed to `--config`
2. `$XDG_CONFIG_HOME/wisp/wisp.toml` on Linux
3. `~/Library/Application Support/wisp/wisp.toml` on macOS
4. `%APPDATA%\wisp\wisp.toml` on Windows

Missing files and unknown keys fall back to defaults, so Wisp runs with no
configuration file at all. A minimal example:

```toml
[font]
family = "IBM Plex Mono"
size_pt = 14.0

[cursor]
style = "block"
blink = true

[theme]
name = "nord"
```

The bundled IBM Plex Mono is used when the configured family is not installed.
See [docs/CONFIG.md](docs/CONFIG.md) for the full configuration reference.

## Key bindings

| Action | Keys |
| --- | --- |
| Copy selection | `Ctrl+Shift+C` |
| Paste | `Ctrl+Shift+V` or middle click |
| Scroll up / down | `Shift+PageUp` / `Shift+PageDown` or mouse wheel |
| Jump to top / bottom | `Shift+Home` / `Shift+End` |
| Select word / line | Double / triple click |
| Extend selection | Right click |

When the child shell exits, pressing any key closes the window.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) describes the crate structure
  and how the PTY, emulator, renderer and window layers fit together.
- [docs/CONFIG.md](docs/CONFIG.md) is the configuration reference.
- [docs/design-reference.html](docs/design-reference.html) is the visual
  design reference for the default theme.

## Contributing

Contributions are welcome. Development happens on the `dev` branch and pull
requests are merged into `main` through it. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the workflow, commit conventions and
the checks your change needs to pass.

## License

Wisp is distributed under the [MIT License](LICENSE).

The bundled IBM Plex Mono fonts in `assets/fonts` are copyright IBM Corp. and
distributed under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
