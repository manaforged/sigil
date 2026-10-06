# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The `sigil-ui`, `sigil-fonts`, `sigil-sign`, and `sigil-kittest` crates share
one version.

## [0.1.0] - Unreleased

First release.

### Added

- `Icon`: the Lucide icon set (98 icons, ISC with the Feather-derived MIT portion), typed, so `icon(Icon::Folder)` draws a stroke icon tinted by the text colour. The nine older built-in icons now come from the same set.
- `tree(nodes)` with `TreeNode`: an explorer or outline tree with icons, toned status, controlled expansion and selection, arrow-key navigation, Left/Right to collapse and expand, Enter to activate, and tree and tree-item accessibility roles.
- `stat(label, value)`: a KPI tile with an optional toned delta, caption, and sparkline trend.
- `card(..).selected(..)` and `.activate(..)`: a selectable card that is a focusable button.
- `chip(..).dot()` and `.leading(node)`; `text_field(..).leading(node)`, `.trailing(node)`, and `.clearable()`.
- `ListRowSlots` gains `leading`, `marker_tone`, `status_tone`, and `badge`, so rows can carry an icon, per-row colours, and a count pill.
- `Tone::token(step)`, `shade()`, `fill()`, and `solid()` give a tone's colour-scale tokens.
- `Shortcut::label()` and `label_for(mac)` print a shortcut as `⇧⌘K` on macOS and `Shift+Super+K` elsewhere.
- A bundled symbol font (a renamed DejaVu Sans subset) covers ⌘ ⌥ ⇧ ⌃, check marks, ✕, arrows, and geometric shapes, so these no longer draw as empty boxes.
- Syntax highlighting for markdown, typescript, go, c and c++, yaml, css, html, and sql.
- `menu_bar().fill()`; `ScrollBuilder` converts into `NodeBuilder`.
- `Task::cancel` stops a running task and sets it back to `Idle`; a
  late result is dropped. `tasks::blocking(f)` runs a subprocess, file
  read, or other blocking work inside a task without stalling a worker.
- `kernel::switch(key).case(..).keep_alive(bool)` shows one view per
  key, and can keep hidden views mounted so they keep their state.
  `kernel::match_status(status)` shows one view per task status.
- `Shortcut::primary` and `events::PRIMARY`: Command on macOS, Control
  elsewhere. `app::menu::accelerator` turns a `Shortcut` into a native
  menu hint, and the native menu types are re-exported under
  `app::menu`, so an app needs no direct `muda` dependency.
- `command_menu` filters its items as you type, ranked with
  `fuzzy_score`. Enter opens a submenu; Escape goes back one level.
- `TabsBuilder::labels` takes a signal or closure, and tabs follow it.
- `task_with(|arg| ..)` and `Task::run_with(arg)`: a task whose runs
  take an argument. `task` is `task_with` over `()`.
- `OwnerHandle::run(f)` runs `f` inside a live reactive owner, so a
  task result reaches the view that started it only while that view is
  mounted; it returns `false` once the owner is disposed.
- `terminal::caps()` returns the capabilities the terminal host probed
  on entry, such as `Caps::backdrop`, so an app picks a light or dark
  theme without a second probe.
- `NodeEventsExt::focus_when(Signal<u64>)` focuses any node or widget
  each time the signal changes.
- `TextEditBuilder::submit_on(SubmitKey::Enter)`: Enter submits and
  Shift+Enter inserts a new line.
- `sidebar`, `splitter`, `virtual_list`, `chart`, `segmented`, and
  `image` take the `Paint` methods.
- `markdown` renders task-list items (`- [ ]`, `- [x]`) as read-only
  check boxes.
- The prelude adds `match_status`, `task_with`, `ModalBuilder`,
  `SortDir`, `SortKey`, `SubmitKey`, and `DiffView`.
- `sidebar(..).resizable(min, max)` adds a drag handle with keyboard
  control.
- `TableState::cycle_sort`, `reverse_sort`, and `select_by` sort and
  move the selection from app keys.

- A retained widget tree built from components, functions that return
  `impl IntoNode`, driven by reactive signals: `signal`, `derived`, and
  effects.
- `sigil::run` opens the tree in a desktop window on Linux, macOS, and
  Windows, and in a canvas on `wasm32`.
- A terminal host (feature `terminal`) paints the same tree on a character
  grid, with screen-reader support and dialogs.
- `sigil::Host` embeds the tree in another engine's event loop.
- A wgpu renderer and a CPU renderer; `SIGIL_BACKEND=cpu` selects the CPU one.
- Text shaping, bidirectional text, IME input, and text editing widgets.
- Layout with flexbox and grid through `LayoutStyle`, `row()`, and `column()`.
- Themes with design tokens and tones.
- Content widgets: `markdown`, `html`, `mermaid`, charts, icons, and images.
- `sparkline(..).fill(SparkFill::Shade)` draws the body of a multi-row
  graph in a dim step of its tone and keeps the top cell of each column
  solid, so a steady high value reads as a line, not a block.
- Accessibility: roles, names, and focus exported to the platform screen
  reader.
- Background tasks with `tasks` and `signal_writer` for results from
  other threads.
- `sigil-sign`: the `sigil-pack` binary packs and signs release builds.
- Each crate's docs.rs page shows its README, and the README examples
  run as doctests.
- `sigil-kittest`: a PNG golden harness for apps that depend on Sigil.
- `sigil-fonts`: the default fonts (Fira Sans, Cascadia Mono, and a Noto
  Color Emoji subset) under the SIL Open Font License 1.1. `sigil-ui` is
  MIT and pulls `sigil-fonts` in only with the `scene` feature.

### Changed

- `kv_grid` takes any iterator of key/value pairs, and values can be reactive.
- `card` returns a `CardBuilder`; it converts into `NodeBuilder`.
- `menu_bar` is as wide as its menus by default.
- Tab content fills the space below the tab strip.
- `chart` fills its parent's width by default and thins its axis labels at small sizes.
- A text input exposes the controls inside it, such as a clear button, to assistive technology.
- Error chips use the same text step as the other tones.
- Host and plumbing items are no longer public where an app has no use
  for them. Each change below is breaking.
  - `AppConfig::window`, `canvas`, `min_size`, `user_zoom`, `frameless`,
    `overlay`, `anchor`, and `resizable` exist only with feature
    `desktop`; `terminal()` already needed feature `terminal`. A build
    without a host's feature no longer compiles a call that selects it.
    Without a call, the host is the window with `desktop`, else the
    terminal.
  - `WindowCommand`, `ResizeDirection`, `WindowOpts`, `Anchor`, and
    `request_window_command` need feature `desktop`, so a terminal-only
    build does not show them.
  - `reactive::tick_intervals`, `is_live`, `owner_by_id`,
    `app::take_exit`, `app::clear_exit`, `events::note_focus_source`,
    `events::take_window_commands`, and the style merge-warning
    counters are crate-private.
- `docs/api.md` lists the methods of every builder and state type.
- One spelling per job. Each change below is breaking.
  - `ButtonBuilder::enabled` is removed; use `disabled(..)`.
  - The `width`/`height(f32)` methods on button, progress, segmented,
    and chart are removed. `WidgetLayoutExt::width` and `height` size
    them and also take `cols(..)` and `rows(..)`.
  - `SplitterOrientation` is removed; `splitter` takes `Orientation`.
  - `badge` is removed; use `chip`.
  - `style::Size` is renamed `ControlSize` and is in the prelude.
  - `font_size` and `line_height_px` take `f32`, like every other
    length. `line_height` is renamed `line_height_px`, so the unit is in
    the name, like `letter_spacing_em`.
  - `visible_when`, `paint_when`, and `invalid_when` take a signal or a
    closure, like `disabled` and `show`.
  - `ChipBuilder::tone` takes a tone, a signal, or a closure;
    `tone_signal` is removed.
  - `diff`, `mermaid`, and `sparkline` take a value, a signal, or a
    closure, and redraw when it changes.
  - `scroll_to_end` and `auto_pin_to_bottom` are replaced by
    `follow_end(bool)` on scroll areas and lists. `follow` is renamed
    `follow_row`.
  - A `virtual_list` row can be any node, for example `text(..)`.
  - Native menus are only at `sigil::app::menu`. The native menu item
    is `app::menu::NativeMenuItem`, distinct from `widgets::MenuItem`.
  - `sidebar_width` is removed; use `sidebar(child).width(..)`.
  - Callbacks use three verbs: `activate()` for a press, `change(value)`
    for an edit, `submit(value)` for a commit. `MenuItem::select` is
    now `activate`, `CommandMenuBuilder::select` is `submit`,
    `DiffBuilder::choose` is `change`, and `VirtualListBuilder::select`
    is `change`.
  - Button tone and fill are independent, so call order does not
    matter. `primary()`, `neutral()`, and `destructive()` set only the
    tone; `solid()`, `soft()`, `ghost()`, and `outline()` set only the
    fill. With no fill, `Primary` and `Error` draw solid and other
    tones draw soft. `button("Go").primary().ghost()` is now a ghost
    button in the primary tone.
  - In `command_menu`, Right and Left no longer open and close
    submenus, because the query field uses them to move the caret.
  - `DiffLayout` is renamed `DiffView`, and `DiffBuilder::layout` is
    `view`, so `.layout` always means `WidgetLayoutExt::layout`.
  - `VirtualListBuilder::focused_row_bind` is renamed `cursor`. The
    cursor row also stays in view; `follow_row` remains for read-only
    targets and wins for scrolling.
  - `chart` and `ChartBuilder::series` take a value, a signal, or a
    closure, and redraw when it changes.
  - `splitter` and `segmented` are no longer `const fn`.
- `text_edit` shows its placeholder while focused and empty, like
  `text_field`. On a terminal its default height is 5 rows, not 20, and
  the dropdown trigger is 3 rows.
- Every widget builder takes the `NodeEventsExt` methods (`key`,
  `key_capture`, `focus_in`, `autofocus`, pointer handlers, and the
  rest), so `virtual_list(..).key(..)` or `button(..).key(..)` needs no
  wrapper container. `WidgetLayoutExt::map_node` runs any `NodeBuilder`
  method on the widget's root node, for example
  `.map_node(|node| node.on_added(..))` to read its id. A widget's own
  handlers run before the ones you add.
- `FrameInputHook` returns `Option<(f32, f32)>`. An embed host returns
  the viewport when its input changed it, for example a resize, and
  `Host::frame` lays out at that size in the same frame. Return `None`
  to keep the viewport passed to `frame`. This is a breaking change.
- A theme set while input runs, for example from a click handler,
  paints in the same frame.
- Focus that the window host gives to the first control on start shows
  no focus ring. Keyboard focus still shows it.
- A component is a function that returns `impl IntoNode`. It runs once
  inside its own reactive owner, so signals it creates live and die with
  its node. `run(config, app)` takes the root component; the closure no
  longer receives `&mut Cx` or returns `NodeId`. `TreeRoot` and `root()`
  are removed. `Host::mount` and `Harness::new` take any `IntoNode`.
  This is a breaking change.
- `row()` and `column()` place children with the theme gap: 8 px in a
  window, one cell in a terminal. A row or column is as wide as its
  content until a child fills. A `Fill` width or a `grow` factor then
  makes it take the width its parent offers, and a row shares that
  width among its children. A row with a `text_field` and a button keeps
  the button on screen; before, the field took the whole width and
  pushed the button past the window edge.
- Style calls write to one store per node, and a later call wins for
  each property it sets: the `Paint` methods, the state variants, and
  `.layout`. A background or border that you set on a widget holds over
  the widget's defaults in every state; `.tone()` keeps the state
  feedback in another palette. `WidgetLayoutExt::padding` is removed,
  so `padding` on a widget is the one typed method. `flex_grow` and
  `shrink` take factors: `flex_grow(2.0)` grows by 2. Before,
  padding on a button left it 30 px wide, `button("A").padding(4.0)`
  did not compile, and a button's hover colour replaced a background you
  set.
- `disabled` on every widget takes a value, a signal, or a closure, so
  `.disabled(move || name.get().is_empty())` needs no `derived`.
  `interactive(..).disabled_signal` is now `disabled`.
- `tabs().tab(label, component)` builds tabs; each component builds once,
  so a tab keeps its state while another is shown. `.active(signal)`
  binds the selected index. `tabs(active, labels)`, `content_for`, and
  `content_for_fn` are removed.
- `field(label, control).error(reactive)` replaces `validated_field`.
- `scroll(content)` measures its content; `.extent(height)` sets a known
  height. `scroll_auto` is removed.
- `announce(text)` and `announce_assertive(text)` are free functions;
  `Cx::announce` and `queue_announcement` are removed. `AppResult`
  defaults its type to `()`.
- `task(make)` runs async work on demand: `run()` shows `Pending`, aborts
  the run it replaces, and drops a result from a replaced run;
  `status()` reads the result. `Status` gains `Idle`, the state before a
  first run. `spawn` stays for a future that starts now. `post`,
  `dispatch_to_owner`, `spawn_to_signal`, `Dispatcher`, and
  `SendableSignal` are removed: write from another thread with
  `signal_writer` or `on_message`.
- `Signal` has five methods: `get`, `peek`, `with`, `set`, and `update`.
  `set_always`, `try_set`, `try_set_always`, `try_peek`, `track`, and
  `into_sendable` are removed. Write `update(|v| *v = next)` where you
  wrote `set_always(next)`, and `with(|_| ())` where you wrote `track()`.
- `Harness` runs async work: it installs a task runtime and applies
  finished results each frame.
- `Harness` finds controls by the label a user sees: `click("Save")`,
  `type_into("Name", "Ada")`, and `expect_text("Saved")`, which waits up
  to `EXPECT_TIMEOUT` for frames and async work. A label picks the
  control over a plain text with the same name and fails on two
  controls. `Target::point` takes the harness.
- `run` takes a title or an `AppConfig`. The `(title, width, height)`
  and `()` conversions and `AppConfig::default` are removed; write
  `AppConfig::new(title).size(width, height)`.
- The prelude holds what an app writes. Embedding and clipboard back-end
  items move to their modules: `sigil::Host`, `sigil::reactive::Runtime`,
  `sigil::events::{EventRouter, ShellEvent, PointerEventInit}`,
  `sigil::layout::{TextMeasure, LayoutError}`, `sigil::kernel::Surface`,
  and `sigil::clipboard::{Clipboard, Delivery, install}`.
- `text_edit` takes `.autofocus()` and `.focus_when(signal)`, as
  `text_field` does, so an app focuses an editor without a node id.

- Every public item has one path. `sigil::prelude` is the only module
  that re-exports items from elsewhere, and it holds the names app code
  uses: widget functions, core types, and the extension traits that
  method calls need. The `events`, `kernel`, `layout`, and `style`
  preludes, the `render` module, and the crate root re-exports are
  removed. This is a breaking change.
- The crate root holds the entry points: `run`, `Host`, `AppConfig`,
  `AppResult`, `Error`, `request_exit`, and `root`. Other application
  items are in `sigil::app`.
- Input and window events are in `sigil::events`. Points, rectangles,
  sizes, and the scale factor are in the new `sigil::geometry`. `Color`
  is in `sigil::style`. `sigil::hal` keeps only the backend traits and
  the types in their signatures. This is a breaking change.
- Smaller moves to one path: `profile` and `bidi` are only at the crate
  root; `NumericAtomKind` is in `style`; `ImageBuilder` is in `kernel`;
  `LayoutError` is in `layout`; `DefaultFonts`, `TextTuning`, and their
  setters are in `app`.
- `widgets::patterns` is removed; its widgets are in `sigil::widgets`.
  The second `tooltip(target, hint)` is removed; use
  `tooltip(hint).child(target)`.

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
- Styling is typed. Paint goes through the `Paint` methods (`bg`,
  `text_color`, `border_color`, `border_w`, `rounded`, `shadow`,
  `opacity`, `font_size`, `font_weight`, `font_family`, and more) and
  the `StateVariants` closures `rest`, `hover`, `focus`, `pressed`, and
  `disabled_style`. Layout goes through `LayoutStyle`, `row()`, and
  `column()`. `LayoutStyle` gains `margin_x`, `margin_y`, `margin_top`,
  `margin_right`, `margin_bottom`, `margin_left`, `gap_x`, and `gap_y`.
  Class strings, the `classes!` macro, `.class(..)`, `parse_classes`,
  `Style`, `.styled()`, and the `Spacing`, `Sizing`, `Flex`, `Fill`,
  `Geometry`, `Typography`, `Visual`, and `Transitions` traits are no
  longer public. `ColorOrToken` no longer converts from a string.
  `Radius::Base` (4 px) and `FontFamily` are new. `paint_when(signal,
  |s| ..)` replaces `class_when` and applies typed paint while a signal
  is true. Widget builders take `.layout(..)`, `.width(..)`, and
  `.height(..)` through `WidgetLayoutExt`. Spacing and size numbers are
  pixels. This is a breaking change.
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

### Removed

- `hairline()`. Use `divider_vertical().height(px)`.

### Migrating from the class-string API

| Before | After |
| --- | --- |
| `.class("text-xs font-mono italic font-bold")` | `.font_size(12.0).font_family(FontFamily::Mono).font_italic(true).font_weight(700)` |
| `.class("text-gray-11 rounded-md bg-* border-* border-l")` | `.text_color(Token::Gray11).rounded(Radius::Md).bg(Token::..).border_color(Token::..).border_left(1)` |
| `.class(Token::OnSurface.text())` | `.text_color(Token::OnSurface)` |
| `.style(Style::new().bg(..).border_width(n))` | `.bg(..).border_w(n)` |
| `.class("w-full h-full")` | `.layout(LayoutStyle::block().fill_width().fill_height())` |
| `.bg("field")` | `.bg(Token::Field)` |
| `badge(..)` | `chip(..)` |
| `sidebar_width(w, child)` | `sidebar(child).width(w)` |
| `.follow(sig)`, `.auto_pin_to_bottom()` | `.follow_row(sig)`, `.follow_end(true)` |
| `.focused_row_bind(sig)` | `.cursor(sig)` |
| `DiffBuilder::choose`, `.layout(DiffLayout::..)` | `change`, `.view(DiffView::..)` |
| `.line_height(px)` | `.line_height_px(px)` |
| `segmented(..).height(f32)`, `chart(..).height(f32)` | `WidgetLayoutExt::height(..)` |
| `sparkline(iter)` | `sparkline(vec)` or a signal |

`Paint` and `WidgetLayoutExt` are in the prelude; a module that does not
glob-import the prelude imports them for `.bg(..)` and `.height(..)`.

### Fixed

- A `field` with an error marks its control invalid and links the message
  as its description, and an open `modal` that traps focus is marked modal
  with the rest of the page hidden from assistive technology. Before, the
  error only changed colour and screen readers could still reach the page
  behind a dialog.
- `chart` draws negative values: bars hang below the zero line, lines go
  below it, and the y axis runs down to a rounded minimum. Before, values
  were clamped at zero and a negative bar drew nothing.
- `number_field` replaces text that does not parse with the stored value on
  Enter or blur. Before, typing "1.234,56" left that text on screen while
  the value silently stayed 1.234.
- A dropped file goes to the deepest `file_drop` handler under the pointer
  (else on the focused node's path) instead of to every handler, so one
  drop no longer lands in two panes.
- `on_scroll` handlers see `ScrollEvent::position` relative to their own
  node, as pointer handlers do. Before, it was in window coordinates, so
  zoom around the pointer in an offset canvas jumped.
- A drag from `drag_source` starts when the first move after the press
  already left the source, as a fast flick does. The press arms the shared
  `DragState`; a source or target under the pointer starts the drag, and
  leaving the source with the button held starts it too.
- `PathNode::fit` lays a path out at its own width and height. Before, it
  made a square of the width, so `path(960, 220).fit()` took 960 x 960.
- `image_bytes` decodes JPEG, as documented. Before, the `image` crate was
  built with PNG only.
- The image cache evicts the least recently used images when it is full.
  Before, it dropped every cached image at 256 entries or 64 MB, so a
  scrolling cover grid decoded the screen just left again.
- `Harness::image` and pixel snapshots paint the theme surface behind the
  tree, as a window does. Before, they cleared to black, so text in the
  default light theme looked unreadable in snapshots.
- Text marked `nowrap` that flex shrinks below its width paints on one line.
  Before, the GPU path wrapped it at the box width while layout kept one
  line, so the extra lines drew over the rows below.
- Ending the scope that started `app::Player` playback no longer panics
  with "RefCell already borrowed", and a thread that ends with a playing
  `Player` no longer aborts the process. `cancel_interval` drops the
  interval's callback after it releases the registry, and does nothing
  once the thread's registry is gone.
- Dragging across `selectable_text` in a terminal selects and copies whole
  cells, including the cell the drag started on; before, the first
  character was dropped. A drag toward the start of the text copies the
  selection; before, it copied nothing.
- Tab runs the capture handlers before it moves focus, so
  `key_capture(Key::Tab, ..)` on an ancestor can own Tab.
- A `follow_row` list that fills after mount stays on its row instead of
  jumping to the end.
- A row that the sort or quick filter hides leaves the table selection,
  so `selected_row` never returns a hidden row.
- The terminal no longer announces live regions that are on screen in
  the first frame, so "Status bar" does not cover the bottom row at
  start.
- The window paints the theme `Surface` behind the tree, so a light
  theme no longer shows a dark clear colour.
- A closeable tab leaves a gap between its label and its close button.

- After `set_theme`, text and controls that did not change kept the
  colours of the old theme until they were rebuilt. Every node now
  repaints in the new theme.
- In the browser, a page that supplies its own canvas laid out the
  first frames at the `AppConfig` size instead of the canvas size, so
  part of the canvas stayed unpainted.
- A table header cell showed its focus ring on pointer focus.
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
- An open modal was laid out again from scratch on every frame that
  changed layout anywhere, so scrolling or typing in a large modal ran
  well past the frame budget. A modal or context menu is now laid out
  again only when it is new or an ancestor was laid out again. The
  `layout.overlay` profile counter counts these passes.
- A hidden tooltip with a transition was marked changed on every layout,
  so any change elsewhere laid the whole window out again from the root.
  Layout now leaves a hidden node alone once its box is cleared.
- Scrolling a `scroll` area laid its whole content out again on every
  wheel step. The content and the scrollbar thumb now move by
  translation, so a step does no layout work.
- Layout measured every flex item again on each change, even when only
  one label in a large tree changed. Measurements now persist across
  frames and are dropped only for the changed node and its ancestors.
  The `layout.intrinsic.miss` profile counter counts the measurements.
- `TableState::replace_rows` rebuilt the cells of every mounted row on
  each call. It now skips the same rows, notifies only when the visible
  order changes, and with `TableState::row_eq` keeps the cells of rows
  that did not change.
- `image_bytes` decoded the image again on every call. Decoded images now
  share one bounded cache with rasterized vector icons.
- Each frame cleared the target in a separate pass before drawing. The
  clear now happens in the first draw pass, which saves one full-target
  store and load per frame on tile-based GPUs.
- Every frame redrew the whole window, even when only a caret, a hovered
  row, or one label changed. The renderer now compares each frame with the
  one before, redraws up to three changed regions into a kept back buffer,
  and copies it to the window. A layer blur, a rotated layer, or a change
  to most of the scene still redraws the whole frame. The
  `gpu.redrawn_pixels` profile counter and `FrameStats::redrawn_pixels`
  report the pixels drawn.

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
- Styling: typed `Paint` methods and state variants, theme colours
  through `Token`, and shared metrics. `parse_theme`, `set_theme`,
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
