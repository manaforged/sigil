# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The `sigil-ui`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [0.1.0] - Unreleased

First release.

### Changed

- The terminal writer sends less per frame. When rows move up or down
  as a block, it scrolls them in place with a scroll region. On a row
  that changed, it rewrites only the changed cells. It clears a blank
  row tail with its background colour, and it sends only the SGR
  attributes that change.
- Focus repair keeps focus on a node that is still live, visible, and
  focusable. Automatic focus applies only when nothing holds focus, and
  it picks the first focusable node in document order. A pending
  `request_focus` or `focus_when` applies before automatic focus, so a
  focusable `virtual_list` earlier in the tree no longer takes focus
  from a later text field.
- `defer(f)` called outside an effect no longer runs at once. Sigil
  queues it until the current dispatch ends: an `on_message` batch,
  event routing, or the frame. The queue runs in one `batch`, so many
  deferred writes cause one refresh. Inside an effect, `defer` still
  runs after the effect body.
- Text runs resolve their colours against the theme of their own node
  when the scene is built, on both hosts. A run colour is lifted to a
  4.5:1 contrast ratio against its background, so inline code, table
  cells with code, and links stay readable in dark and light themes.
- Inline code in markdown uses the `on-surface` text colour on a
  `surface-subtle` background. A code span moves to the next row as a
  whole instead of breaking at a `/` inside it.
- Terminal markdown headings differ by level: level 1 is bold and
  underlined, level 2 is bold in the accent colour, and level 3 is bold
  in the text colour.
- Without OSC 8 hyperlinks, a terminal shows a markdown link as
  `text (url)` with a muted URL. It shows the URL once when the text
  is the URL.
- Italic text in the window uses a synthesized oblique face.
- In the terminal, `notice_band` is one row: a tone marker and the
  text on a tinted row. It was five rows tall with its padding and
  border. The window form is unchanged.

### Fixed

- A split diff gave each side half the width, so a long added line was
  cut while the removed side had spare room. Each side now takes width
  in proportion to its widest line.
- A long markdown link broke right after `https://`. A URL now wraps
  only after `/`, `?`, `&`, or `=` past the host, and the plain-text
  fallback `text (url)` wraps the same way.
- A markdown thematic break (`---`, `***`, `___`) showed as literal
  text. It now draws a dim rule across the column.
- A cached subtree that moved, for example after a scroll, kept its
  terminal border at the old cells. Table frames and quote bars now
  move with their content.
- The renderer always asked wgpu for the low-power adapter, so a
  hybrid-GPU machine could not use its discrete GPU. It now honours
  `WGPU_POWER_PREF=high|low|none`; the default stays low power.

### Added

- `highlight(language, source)` and `hl_spans(language, source)`: one
  syntax highlighter for rust, sh, toml, json, python, js/ts, and diff.
  Rules live in a TOML file and colours come from theme tokens. Other
  languages stay monochrome. Markdown code fences use it on both hosts,
  and `code_edit(..).highlights(..)` accepts `hl_spans` output.
- `HostCaps` and `Host::set_caps`: widgets read what the host can show
  through `HostCaps::current()`.
- Hosts. `sigil::run(config, tree)` opens a window (feature `desktop`,
  the default) or runs the same tree in a terminal with
  `AppConfig::new(title).terminal()` (feature `terminal`). `Host`
  embeds the tree in another event loop: it routes events, lays out,
  returns a `CommandScene`, and reports `surface(key)` holes.
- Window renderer on wgpu, with a CPU renderer. `SIGIL_BACKEND`
  selects `auto`, `wgpu`, or `cpu`. `auto` falls back to the CPU
  renderer when the GPU surface fails.
- Terminal host: Kitty, iTerm2, and Sixel image protocols with a glyph
  fallback, colour depth detection that honours `NO_COLOR`, bracketed
  paste, mouse capture, `suspend`, `set_title`, `bell`, `focused`, an
  OSC 52 clipboard, and `run_with` hooks for an external event pump.
- Reactive state: `signal`, `derived`, `effect`, `batch`,
  `EventChannel`, and `interval` for periodic callbacks.
- Tasks: `spawn` runs async work into a `Signal<Status>`.
  `signal_writer` and `post` move results from other threads to the
  UI thread.
- Widgets: controls, fields, collections, documents (`markdown`,
  `html`, `mermaid`, `diff`, `code_edit`), data display (`chart`,
  `gauge`, `heatmap`), application chrome, and overlays. The
  [widgets guide](docs/guide/widgets.md) lists each group.
- Layout on flexbox and grid, with `rows`, `cols`, and `em` text units.
- Styling: `classes!` utility classes, typed theme colours through
  `Token`, and shared metrics. `parse_theme`, `set_theme`,
  `watch_theme`, and `AppConfig::theme_file` load a TOML theme and
  reload it when the file changes.
- Accessibility through AccessKit: roles, names, and live regions.
  `Host::accessibility_tree` exposes the tree to tests and agents.
- Clipboard: `copy`, `paste`, and `install`. `OsClipboard` (feature
  `clipboard`) uses the system clipboard. `clipboard-image` reads
  images.
- `open` passes `http`, `https`, and `mailto` URLs to the OS handler.
- Native dialogs, menus, and tray icons (features `dialog`, `menu`,
  `tray`), audio playback (`audio`), and pseudo-terminal sessions
  (`pty`).
- `sigil-sign`: `macos_app` and `windows_dir` build app bundles.
  `sign_file` signs update packages with minisign and binds the file
  name and version in the trusted comment. `verify_file` checks the
  signature, the file name, and an optional expected version. The
  `.sig` file is the plain minisign envelope. The `sigil-pack` command
  runs the same steps, with `sign`, `verify`, and `genkey DIR`.
- `sigil-kittest`: visual golden tests that render a widget offscreen
  in a chosen interaction state and compare it with a committed PNG.
