# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The workspace publishes in lockstep: every published `sigil-*` crate shares one version.

## [0.1.0] — 2026-09-17

First public release. The facade publishes as `sigil-gui` (the `sigil` name was
already taken on crates.io); its library is still named `sigil`, so
`use sigil::prelude::*;` remains the single import user code needs.

### Added

- `mermaid` renders flowcharts and state diagrams (layered layout, box-drawing
  edges with labels, back edges through side lanes), sequence diagrams
  (lifelines, arrows, notes, `loop`/`alt`/`opt` frames, `autonumber`), and
  pie charts as bars; other kinds show the source in a frame.
  `mermaid_draw` gives the lines, `mermaid_diagram` says whether a source
  is a supported kind.
- Terminal pictures: an `image` node in the terminal host draws through the
  Kitty graphics protocol (Kitty, Ghostty, WezTerm, Konsole) or iTerm2
  inline images (iTerm2, Warp), detected from the environment, and as
  half-block cells elsewhere. `SIGIL_GRAPHICS=kitty|iterm2|none` overrides
  the detection; tmux disables it. `Painted` carries the image source and
  an optional `ImageCrop` (source rows), so a partly visible picture shows
  its visible part without re-sending pixels.
- Signal-graph reactivity (`signal`, `derived`, `effect`, `batch`) with a
  measured, budgeted write path.
- Owned incremental layout engine, benchmarked against a Taffy reference corpus.
- Slug vector text pipeline plus hinted raster glyph atlas; IME-ready text
  editing; font fallback with Arabic, Hebrew, CJK, and emoji routing
  (Devanagari, Khmer, Thai, and Lao shaping tests are ignored; their
  fixtures are missing).
- Three-layer style system: `classes!` runtime-parsed utility classes,
  a `Style` struct API, and `style_reactive` signal binding; 12-step design
  tokens; tone × variant theming with WCAG-aware foregrounds.
- Widget catalog: avatar, badge, button, card, chart, checkbox, chip,
  code_edit, color_field, combobox, command_menu (a nested action
  tree, not a searchable Cmd-K palette), date_field, divider_horizontal, divider_vertical, dot, dropdown, field, floating_text, gauge,
  icon, icon_button, image_rgba, interactive, heatmap, markdown, markdown_edit, menu,
  mermaid, modal, number_field, panel, path, progress, radio_group, scroll,
  sheet, sidebar, slider, splitter, table (headless state), tabs, text_edit,
  text_field, toast, toast_queue, toggle, tooltip, turn_strip,
  validated_field, virtual_list. `button.pending(Signal<bool>)` paints
  disabled while a job is in flight. Native alerts are
  `sigil::dialog::message` / `confirm` (rfd; GTK on Linux).
- End-to-end AccessKit accessibility and IME.
- `sigil::launch` one-call app entry with DPI-aware logical sizing.
  Installs the OS clipboard so text fields share `sigil::clipboard`.
- Application host: `html()` document ingest (same nodes as `markdown`),
  `transport` + `sigil::audio::Player` (feature `audio`), `terminal` +
  `sigil::pty::Session` (feature `pty`), `surface(key)` layout holes
  with `Host::surfaces` for native views. Not a webview.
- Headless testing (`sigil::testing`) and a golden-image visual harness.
- Terminal host (crate `sigil-tui`, `sigil_tui::run`): the same tree, widgets,
  and events as the desktop window, painted onto a character grid.
  - `Screen` holds the buffers a host running its own loop keeps between
    frames; `render` writes the difference between two of them. A frame that
    changed nothing sends the terminal no bytes.
  - Colour resolves through the same code as the window host
    (`sigil::style::Theme::resolve_color`), so a badge is the theme's own swatch. 24-bit
    where `COLORTERM` advertises it, the 256-colour cube otherwise.
  - Mouse and wheel input, mapped onto `ShellEvent::Pointer` and `::Scroll`:
    hover, click-to-select, wheel scrolling, and header sort work as they do
    on desktop.
  - `sigil_tui::every` runs a callback on the frame clock; `sigil_tui::work`
    runs a job on a worker thread and hands the result back on a frame,
    with no async runtime.
  - `sigil_tui::present` and `sigil_tui::step` drive the host in tests.
    `sigil_tui::{click, pointer, wheel}` build pointer events without crossterm.
  - `sigil::request_exit` returns from `launch` and `sigil_tui::run` after the
    current frame.
  - `sigil::style::grid` measures control chrome in terminal cells, so a
    button is one line rather than four.
  - Text is placed by display width, so a CJK glyph keeps its two columns.
  - A panic restores the terminal before it prints, rather than leaving a
    blank screen and no reason.
- Text editors accept native copy, cut, paste, select-all, undo, and
  redo shortcuts (Control everywhere, Command on macOS). A terminal host
  reads queued bracketed paste; it does not install an OS clipboard.
- `TextContent::resolve_into` and `refresh` resolve text into a caller's
  buffer, and report whether it changed, without allocating for fixed text.
- `Host::repaint` emits the paint scene and reports whether anything moved.

### Changed

- GitHub Actions runs on pushes to main, pull requests, release tags,
  and manual dispatch.

### Known issues

- A form field can render its label twice after a save and reopen. The
  duplication is intermittent and the cause is not isolated.
