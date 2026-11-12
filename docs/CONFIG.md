# Wisp configuration reference

Wisp loads its configuration from the first existing location:

1. `--config <path>` command line argument
2. `$XDG_CONFIG_HOME/wisp/wisp.toml` (Linux)
3. `~/Library/Application Support/wisp/wisp.toml` (macOS)
4. `%APPDATA%\wisp\wisp.toml` (Windows)

Missing files and unknown keys fall back to defaults; invalid values abort startup
with a descriptive error.

## Example

```toml
[window]
padding.x = 24
padding.y = 20
opacity = 0.85
title = "wisp"
min_width = 320
min_height = 180

[font]
family = "IBM Plex Mono"
size_pt = 14.0
line_height = 1.3

[terminal]
shell = "/bin/zsh"
scrollback = 10000

[cursor]
style = "block"
blink = true
blink_interval_ms = 500

[theme]
name = "wisp"

[theme.colors]
background = "#1A222C"
foreground = "#C4C9D2"
cursor = "#E1E3E9"
```

## Sections

### window

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `padding.x` / `padding.y` | u16 | 24 / 20 | Grid padding in physical pixels |
| `opacity` | f32 | 0.85 | Window opacity, 0.0 - 1.0 |
| `title` | string | `wisp` | Initial window title; OSC title changes override it |
| `min_width` / `min_height` | u32 | 320 / 180 | Minimum window size in pixels |
| `decorations` | bool | true | Show OS window decorations |
| `maximized` | bool | false | Start the window maximized |

### font

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `family` | string | `IBM Plex Mono` | Font family. Resolved via the system; the bundled IBM Plex Mono is used as fallback |
| `size_pt` | f32 | 14.0 | Font size in points, clamped to 6.0 - 72.0 |
| `line_height` | f32 | 1.3 | Line height multiplier, clamped to 1.0 - 3.0 |

### terminal

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `shell` | string | platform default | Shell command. Falls back to `$SHELL`, then `bash`/`sh` on Unix or `powershell` on Windows |
| `scrollback` | u32 | 10000 | History lines kept per terminal, max 1,000,000 |

### cursor

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `style` | `block` / `beam` / `underline` | `block` | Default cursor shape; applications can override it via escape sequences |
| `blink` | bool | true | Whether the cursor blinks when idle |
| `blink_interval_ms` | u64 | 500 | Blink interval, clamped to 100 - 2000 |

### theme

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | string | `wisp` | One of the built-in themes: `wisp`, `nord`, `gruvbox-dark`, `dracula`, `one-dark` |
| `colors` | table | none | Inline palette overriding the theme entirely |

Palette keys: `background`, `foreground`, `cursor`, `cursor_text`,
`selection_background`, `selection_foreground`, `black`..`white`,
`bright_black`..`bright_white`. Values are `#RGB` or `#RRGGBB`.

## Keybindings

| Action | Keys |
| --- | --- |
| Copy selection | Ctrl+Shift+C |
| Paste | Ctrl+Shift+V or middle click |
| Extend selection | Right click or left drag |
| Word / line select | Double / triple click |
| Scrollback | Mouse wheel or Shift+PageUp/PageDown |
| Scroll to top / bottom | Ctrl+Shift+Home / Ctrl+Shift+End |

## Behavior notes

- Shell input passes through a full xterm-style encoder: control chars, arrows,
  navigation keys and Alt-prefixed characters.
- Bracketed paste is honored automatically for both IME commits and clipboard pastes.
- OSC 52 clipboard requests are applied to the system clipboard.
- In the alternate screen the wheel sends arrow keys instead of scrolling history.
