# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The workspace publishes in lockstep: every `sigil-*` crate shares one version.

## [0.1.0-alpha.1] — Unreleased

First public alpha. The facade publishes as `sigil-ui` (the `sigil` name was
already taken on crates.io); its library is still named `sigil`, so
`use sigil::prelude::*;` remains the single import user code needs.

### Added

- Signal-graph reactivity (`signal`, `derived`, `effect`, `batch`) with a
  measured, budgeted write path.
- Owned incremental layout engine, benchmarked against a Taffy reference corpus.
- Slug vector text pipeline plus hinted raster glyph atlas; IME-ready text
  editing; font fallback with CJK/emoji routing.
- Three-layer style system: `classes!` compile-time-checked utility classes,
  a `Style` struct API, and `style_reactive` signal binding; 12-step design
  tokens; tone × variant theming with WCAG-aware foregrounds.
- Widget catalog: button, checkbox, chip, code_edit, command_menu, divider,
  dot, dropdown, floating_text, gauge, icon, menu, modal, number_field, panel,
  progress, radio_group, scroll, slider, splitter, table (headless state),
  tabs, text_edit, text_field, toggle, tooltip, turn_strip, virtual_list.
- End-to-end AccessKit accessibility and IME.
- `sigil::launch` one-call app entry with DPI-aware logical sizing.
- Headless testing (`sigil::testing`) and a golden-image visual harness.
- Terminal host (`sigil::run_tui`, crate `sigil-tui`): the same tree, widgets,
  and events as the desktop window, painted onto a character grid.
  - `Screen` holds the buffers a host running its own loop keeps between
    frames; `render` writes the difference between two of them. A frame that
    changed nothing sends the terminal no bytes.
  - Colour resolves through the same code as the window host
    (`sigil_style::resolve`), so a badge is the theme's own swatch. 24-bit
    where `COLORTERM` advertises it, the 256-colour cube otherwise.
  - Mouse and wheel input, mapped onto `ShellEvent::Pointer` and `::Scroll`:
    hover, click-to-select, wheel scrolling, and header sort work as they do
    on desktop.
  - `sigil::every` runs a callback on the frame clock; `sigil::work` runs a
    job on a worker thread and hands the result back on a frame, with no
    async runtime.
  - `sigil_style::grid` measures control chrome in terminal cells, so a
    button is one line rather than four.
  - Text is placed by display width, so a CJK glyph keeps its two columns.
  - A panic restores the terminal before it prints, rather than leaving a
    blank screen and no reason.
- `TextContent::resolve_into` and `refresh` resolve text into a caller's
  buffer, and report whether it changed, without allocating for fixed text.
- `Host::repaint` emits the paint scene and reports whether anything moved.

### Fixed

- `table()` cells read their row data reactively. `replace_rows` changed the
  data and the screen kept its first frame.
- `chart()` asked for a `border-line` theme token that does not exist, which
  panics in a debug build on either host.

### Removed

- GitHub Actions orchestration, in favor of direct verification on owned
  native builders.
