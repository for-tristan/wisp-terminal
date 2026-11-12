# Contributing to Wisp

Thank you for considering a contribution. This document describes how the
repository is organized, how to get a build running, and what a change needs
to pass before it can be merged.

## Getting started

1. Install Rust 1.85 or newer through [rustup](https://rustup.rs).
2. Clone the repository and build:

   ```sh
   git clone https://github.com/for-tristan/wisp-terminal.git
   cd wisp-terminal
   cargo build --release
   ```

3. Run the test suite to confirm your checkout is healthy:

   ```sh
   cargo test --workspace
   ```

Platform-specific requirements (FreeType and fontconfig on Linux, Visual
Studio Build Tools on Windows) are listed in the
[README](README.md#requirements).

## Branching and pull requests

- Development happens on the `dev` branch. Base your work on `dev`, and open
  pull requests that target `dev`. Changes reach `main` only through a merge
  of `dev`, which keeps `main` always shippable.
- Keep one logical change per pull request. A rendering fix and a documentation
  rewrite should be separate pull requests.
- Update the documentation that your change affects: user-facing behavior
  belongs in `README.md` and `docs/CONFIG.md`, internal design in
  `docs/ARCHITECTURE.md`, and every user-visible change deserves a line in
  `CHANGELOG.md`.

## Commit messages

- Use short, lowercase, imperative summaries, for example
  `add keyboard scrolling with shift page keys` or
  `rasterize glyphs at display scale and snap text to pixel grid`.
- The summary line should stand alone; put details in the body when needed.

## Code style

- The codebase deliberately contains no inline comments. Write code that
  explains itself through naming and structure instead of annotating it.
- Run `cargo fmt --all` before committing; the CI gate rejects unformatted
  code.
- Keep `cargo clippy --workspace --all-targets` clean.

## Checks before opening a pull request

Run all three locally; CI runs the same set on Linux, macOS and Windows:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

A pull request that changes rendering behavior on one platform should note
which platforms you were able to verify by hand, so reviewers with access to
the others can help cover the gap.

## Project layout

| Crate | Responsibility |
| --- | --- |
| `crates/wisp-config` | `wisp.toml` parsing, defaults, themes |
| `crates/wisp-core` | PTY management, ANSI/VT emulation state, snapshots |
| `crates/wisp-render` | wgpu renderer, fonts, glyph atlas, shaders |
| `crates/wisp-app` | The `wisp` binary: window, event loop, input |

Architecture details, including how data flows from the PTY reader thread to
a rendered frame, are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Reporting issues

When reporting a bug, include your operating system, display scaling
percentage, GPU/driver, the Wisp commit you built from, and the steps to
reproduce. Log output from `wisp -v` is usually enough to locate the failing
layer.
