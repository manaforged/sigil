# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The `sigil-ui`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [0.1.0] - Unreleased

First release.

### Added

- A retained widget tree built with `Cx` and builder helpers, driven by
  reactive signals: `signal`, `derived`, and effects.
- `sigil::run` opens the tree in a desktop window on Linux, macOS, and
  Windows, and in a canvas on `wasm32`.
- A terminal host (feature `terminal`) paints the same tree on a character
  grid, with screen-reader support and dialogs.
- `sigil::Host` embeds the tree in another engine's event loop.
- A wgpu renderer and a CPU renderer; `SIGIL_BACKEND=cpu` selects the CPU one.
- Text shaping, bidirectional text, IME input, and text editing widgets.
- Layout with flexbox and grid, class strings, and builder methods.
- Themes with design tokens and tones.
- Content widgets: `markdown`, `html`, `mermaid`, charts, icons, and images.
- Accessibility: roles, names, and focus exported to the platform screen
  reader.
- Background tasks with `tasks` and `signal_writer` for results from
  other threads.
- `sigil-sign`: the `sigil-pack` binary packs and signs release builds.
- `sigil-kittest`: a PNG golden harness for apps that depend on Sigil.

### Changed

- The terminal host picks colour depth from more terminals: `TERM`
  `xterm-ghostty`, `xterm-kitty`, `wezterm`, `alacritty`, `foot`,
  `contour`, and `*-direct` get truecolor. `SIGIL_COLOR` forces the depth.
- With 16 colours, toned text and fills use the tone's ANSI colour, and a
  saturated exact colour never maps to grey or black.
- Inside tmux or screen, the terminal host ignores environment variables
  inherited from the outer terminal, and keeps legacy keys unless the
  terminal answers the keyboard probe.
- The terminal host maps glyphs through `terminal::Glyphs`: ASCII for a
  non-UTF-8 locale, `TERM=dumb`, `TERM=vt100`, or `SIGIL_ASCII=1`, and a
  reduced set for the Linux console.
- `app::set_ambiguous` counts East Asian
  ambiguous characters as two cells. `SIGIL_AMBIGUOUS_WIDE=1` sets `Caps::ambiguous`.
- The terminal cursor shows only at a text caret. Before, an app with no
  text input showed a block cursor at the focused node or the top-left cell.
- Terminal mouse capture asks for clicks and drags (1000, 1002, 1006), not
  all motion (1003). `terminal::MouseTracking`, `set_mouse_tracking`, and
  `AppConfig::mouse_tracking` pick the mode; `Motion` turns hover back on.
- Ctrl+V in a text input pastes on a local macOS terminal. Over OSC 52,
  where the clipboard cannot be read, the key goes on to the app.
- The terminal probe waits up to 1 s for the terminal's reply and reads it
  to the end, so a slow reply never reaches the app as keys. A stray OSC or
  APC reply in the input is dropped.
- A multi-line burst of typed text in one read arrives as one paste event,
  for terminals without bracketed paste.
- After SIGTERM, SIGHUP, or SIGINT restores the terminal, the host paints no
  more frames.
- The native task runtime starts on the first spawned task, not at app
  start. An app that never spawns a task owns no worker threads.
  `AppConfig::task_workers` and `AsyncRuntime::with_workers` take a
  `tasks::TaskWorkers` to fix the worker count.
- `derived` skips unchanged values: it needs `T: PartialEq`, and its
  readers do not re-run when the new value equals the old one.
  `derived_eq` is removed; call `derived`. This is a breaking change.
- `text(source)` is the one text node. `source` is a `&str`, a
  `String`, a `Signal<T: Display>`, or a closure `Fn() -> String`.
  `text_live`, `TextContent::compute`, and `widgets::label` are
  removed; write `text(f)` and `text(source).nowrap()`. This is a
  breaking change.
- The `classes!` macro is removed; write `.class("p-16 bg-surface")`.
  `parse_classes` is in the prelude. An unknown class panics in a debug
  build with the nearest valid names and logs one warning per name in a
  release build. This is a breaking change.
- Spacing and size numbers in class strings are pixels, the unit of the
  builder methods: `p-16` is 16 px, as `.padding(16.0)` is. Multiply an
  old class number by 4 to keep the same size (`p-4` becomes `p-16`).
  This is a breaking change.
- The app root fills the viewport on both hosts when its width or
  height is auto. A root no longer needs `fixed_size` to match the
  window.
- `TextFieldBuilder::focus_when` no longer focuses on mount when the
  signal is already nonzero; use `.autofocus()`. This is a breaking
  change.
- `NodeBuilder::on_added` runs every hook it receives, in order, instead
  of only the last one.
- Scene types have one path: `sigil::scene`. `sigil::renderer` no
  longer re-exports `CommandScene`, `SceneBuilder`, `SceneCommand`, the
  draw payloads, the style structs, `emit_scene`, or `shape_glyph_run`,
  and `renderer::style` and `renderer::prelude` are removed. `sigil::render`
  keeps its re-exports for embedders. This is a breaking change.
- `sigil::layout` exports `Display`, `Position`, `FlexDirection`,
  `JustifyContent`, `AlignItems`, `AlignSelf`, `LengthPercentage`, and
  `LengthPercentageAuto` under those names only. The `Style*` aliases
  are removed. This is a breaking change.
- Renderer, layout, and text plumbing is no longer public:
  `renderer::submit_clear`, `renderer::scenes`, `SlugTextPipeline`,
  `DrawParams`, `LayerPool`, `LayerHandle`, `FrameStats`,
  `last_frame_stats`,
  `BUILTIN_SPRITE_ATLAS_ID`, the `WgpuRenderer` pipeline-count and
  headless methods, `layout::LayoutDrive`, `NodeHandle`, `ResolvedBox`,
  `TextEngineMeasure`, `SizeReactive`, `TextStyleCapture`,
  `set_text_geometry`, `with_text_geometry`, `layout::style_adapter`,
  `text::color_glyph`, `text::glyph`,
  `text::FontCollection`, `OwnedTextEngine::fonts`, `text::SansWeight`,
  `shell::WAKE_DELIVERED_COUNT`, `events::invoke_node`,
  `icons::QuadCurve`, `icons::cubic_to_quadratics`,
  `ANIMATION_REGISTRY_MISS`, `binding_animate::miss_counter`, and
  `unsupported_style_warning_count_for`. Test support stays reachable
  with feature `kittest`. This is a breaking change.
- `text::segment`, `WgpuRenderer::submit_world`, `WorldSubmission`, and
  `RendererError::InvalidViewProj` are removed. `submit_world` checked
  its arguments and drew nothing; use `unicode-segmentation` in place of
  `text::segment`. This is a breaking change.
- `sigil::profile::begin_frame` takes the host's `BackendStatus`, and
  `Frame::backend` holds the status of the host that drew the frame.
  `profile::record_backend` and `profile::backend` are removed, so a
  window host and a terminal host in one process no longer overwrite
  each other's status. This is a breaking change.
- `sigil::profile::interact` takes a `profile::Cause`, and
  `Frame::cause` is a `Cause` in place of a string. This is a breaking
  change.
- A colour transition resolves theme tokens through
  `Theme::resolve_color`, so runtime overrides and theme-file values
  animate the same as they paint. `Theme` implements `Clone`.
- Dialogs are async. `FileDialog::open_file`, `open_files`,
  `pick_folder`, `save_file`, and `dialog::message` / `confirm` return a
  `Signal<Status<..>>` and no longer block the frame loop. The window
  host runs the native dialog on the task runtime. The terminal host
  shows an in-app, keyboard-driven file picker and alert. Feature
  `dialog` no longer pulls in the desktop stack: it gives the API and
  the in-app picker on every host. The new feature `native-dialog`
  adds the OS dialog through rfd. This is a breaking change.
- Unused public methods are removed: `Cx::last_child`,
  `Easing::cubic_bezier` (use `Easing::Bezier(CubicBezier::new(..))`),
  `LayoutEngine::parent_content_box`, `WinitShell::place_overlay`,
  `LayoutStyle::clips_paint`, `CornerRadii::corner_for`,
  `StyleReactiveEffect::current_slice`, `GlyphRun::has_positioned_glyphs`,
  `TableColumn::min_width_px`, `TextContent::resolve_into`, and
  `Host::route_all`. This is a breaking change.
- `sigil::reactive::register_interval` is removed. Call
  `sigil::reactive::interval`. This is a breaking change.
- The test clock hooks `sigil::reactive::__test_set_clock`,
  `__test_advance_clock`, and `__test_clear_clock` need the new feature
  `test-clock`. The unused feature `capture` is removed.
- `sigil_sign::verify_file` takes the expected version as `&str` and
  returns `()`. `sigil-pack verify` requires `--version`, so an older
  signed package is refused. This is a breaking change.
- `sigil_sign::sign_file` returns `SignError::PasswordRequired` when an
  encrypted key has no password.
- `sigil::terminal::run_with` is removed. Pass `Hooks` through
  `AppConfig::terminal_hooks` and call `sigil::run`. `terminal::step` and
  `terminal::present` render through `Screen::frame`, the same path as
  the terminal host. This is a breaking change.
- `sigil::terminal::Hooks` has a new field, `region_rows`. Code that
  builds `Hooks` with a struct literal must add it or use
  `..Hooks::default()`. This is a breaking change.
- Scheduler::spawn returns tasks::TaskHandle instead of a tokio JoinHandle.
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

- Code hot reload behind the opt-in `hot` feature. Under
  `dx serve --hot-patch`, `sigil::run` patches the binary crate through
  `subsecond` and mounts the root again without a restart. Signals made
  outside the root closure keep their values, and focus returns to the
  node with the same role and name path. `Host::mount_hot` gives the same
  to an embed host. Not on wasm. See [Hot reload](docs/guide/hot-reload.md).
- `show(cond, then, otherwise)` and `for_each(items, key, render)` in
  the prelude. `for_each` reconciles by key, so a row whose key stays
  keeps its node and state.
- `.autofocus()` on `NodeBuilder` (through `NodeEventsExt`) and on
  `TextFieldBuilder` focuses the node when it mounts.
- Inline terminal host. `AppConfig::inline(true)` draws the live region
  in the normal screen below the prompt instead of the alternate screen,
  so earlier output stays in scrollback. `Hooks::region_rows` sets the
  height of the live region each frame. `sigil::terminal::print_above`
  writes lines above the live region into scrollback.
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
  OSC 52 clipboard, and `terminal::Hooks` for an external event pump.
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
  `Host::accessibility_tree` exposes the tree to tests and automation.
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
