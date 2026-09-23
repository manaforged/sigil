# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The workspace publishes in lockstep: every published `sigil-*` crate shares one version.

## [Unreleased]

Not tagged. Not on crates.io. The repository is still private.

### Added

- `sigil-kittest`: the visual-golden PNG harness is now a crate
  dependents can use as a dev-dependency. Same API as the old
  `tests/golden_support` module: `render_widget_to_png`,
  `assert_png_matches_golden`, `golden_path!`, `assert_goldens_differ!`,
  `InteractionMode`. `golden_path!` resolves `tests/goldens/` under the
  caller's manifest dir.
- `agent_driver` example: mounts a `Host` headless, reads the AccessKit
  tree, finds a button by name, and clicks it through `EventRouter`.
- Book chapter "Driving Sigil from an agent" covering `Host`,
  `EventRouter`, `accessibility_tree`, `sigil_tui::present`/`step`, and
  `sigil-kittest`.
- All `sigil` modules are now visible in rustdoc; nothing is
  `#[doc(hidden)]` except `__drive_test_frame`.
- `sigil-sign` and `sigil-pack`: macOS `.app` and Windows app directory,
  minisign signing. Extra files use `--resource-from` / `--resource-to`.
  Paths must stay inside the bundle. Symlinks and Windows reparse points
  are refused. `sigil-pack sign` writes a base64 minisign `.sig`. Password
  is `SIGIL_PACK_PASSWORD`. Ad-hoc `codesign --sign -` is not notarization.
- `sigil-pack recipe` validates a module recipe against the client schema,
  signs it, and writes the staging `manifest.json` the release host consumes.
  `genkey` writes a development keypair and refuses to overwrite one.

### Changed

- `markdown` images load only from `data:` base64 URIs. The widget no
  longer reads a local file path, so untrusted text cannot make it read
  a file. Each image decodes once.
- `AppConfig::esc_exits` defaults to `false`. Escape reaches widgets
  (modal, menu, dropdown) instead of quitting the app. Call
  `.esc_exits(true)` to restore the old behaviour.
- `sigil-pack` replaces an existing output bundle only when it wrote
  that bundle. A macOS bundle now carries a `SigilPack` key in
  `Info.plist`. Repack over an older bundle by deleting it first.
- `genkey` creates the secret key with mode 0600. `sigil-pack sign`
  refuses to write the `.sig` through a symlink.

### Removed

- `sigil-pack recipe` and the recipe schema: module-recipe policy is
  product-owned, not library API. The same minisign `.sig` flow stays
  under `sigil-pack sign`.

### Fixed

- `text_field` shows a value that the app sets, for example a field
  cleared after save. The next keystroke no longer restores the old
  text.
- `notice_band(Tone::Warn)`, the empty `chart`, and `code_edit` used
  theme tokens that the default theme does not define. They panicked
  in a debug build.

### Known issues

- Intermittent doubled label after save and reopen in a form (issue #5).

## [0.1.0] — 2026-09-17

Library contents for the first public tag. That tag is not cut yet. The
facade publishes as `sigil-gui` (the `sigil` name was already taken on
crates.io); its library is still named `sigil`, so
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
