# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `sigil-ui`, `sigil-fonts`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [0.1.0] - Unreleased

First public release.

### Added

#### Core

- A retained widget tree built from component functions whose results implement `IntoNode`. `sigil::run` runs the application on the selected host.
- `Host` embeds the same tree in another engine's event loop, with event routing, layout, drawing, and host-rendered `surface` regions.
- Reactive state through `signal`, `derived`, and `effect`. `batch` groups writes, and `interval` runs periodic callbacks.
- Conditional views through `show` and `switch`, with optional state preservation for hidden views.
- Keyed collections through `for_each`, preserving each item's node and state while its key remains.
- Async work through `task`, `task_with`, and `spawn`, with reactive status and cancellation for tasks. `match_status` renders loading, result, and error views.
- `tasks::blocking` runs blocking work inside async tasks. `signal_writer` and `tasks::on_message` deliver results from other threads to the UI thread.
- Flexbox and grid layout through `LayoutStyle`, `row`, and `column`, with pixel sizes and text units `rows`, `cols`, and `em`.
- Keyboard, pointer, scroll, paste, and gesture handlers through `NodeEventsExt`. `Shortcut` parses and displays key combinations, with `Shortcut::primary` selecting Command or Control for the platform.

#### Widgets

- Push buttons through `button` and `icon_button`, with independent tone and fill choices, disabled states, and activation callbacks.
- Boolean controls through `checkbox` and `toggle`.
- Single-choice controls through `radio_group` and `segmented`.
- Numeric sliders through `slider`, with step sizes and horizontal or vertical orientation.
- Single-line input through `text_field`, with password masking, leading and trailing content, and a clear button.
- Multi-line input through `text_edit`, with selection, reactive focus, and configurable submission keys.
- Numeric input through `number_field`, with range limits and locale-aware parsing.
- Date input through `date_field`, using the theme's date format.
- Colour input through `color_field`, with a swatch and hexadecimal entry.
- Form labels and validation through `field`, with required markers, error messages, and accessible descriptions.
- Choice fields through `dropdown` and searchable `combobox`.
- Sortable, filterable tables through `table` and `TableState`, with row selection and configurable columns.
- `virtual_list` builds only visible rows, supports measured row heights, and can follow a row or the end of the list. Measured lists with stable keys preserve the visible item when rows are inserted above it.
- Explorer and outline views through `tree` and `TreeNode`, with controlled expansion, selection, icons, and keyboard navigation.
- `tabs` keeps each tab's component state while another tab is visible.
- Application structure through `app_shell`, titled `panel` containers, and `status_bar`.
- Selectable `card` containers with keyboard and pointer activation.
- Resizable sidebars and panes through `sidebar` and `splitter`.
- `scroll` measures its content or accepts an explicit extent.
- Popup and context menus through `menu` and `context_menu`, plus an in-window `menu_bar`.
- `command_menu` filters and ranks commands as you type, with nested submenus and keyboard navigation.
- Modal dialogs through `modal` and side panels through `sheet`, with focus trapping and configurable dismissal.
- Hover hints through `tooltip` and keyboard hints through `hint_overlay`.
- Transient messages through `toast` and `toast_queue`.
- Status and progress indicators through `chip`, `dot`, `spinner`, `progress`, and `gauge`.
- Metric tiles through `stat`, with optional change indicators, captions, and trend lines.
- Bar, line, and candlestick plots through `chart`, with negative values, zoom, pan, and pointer readouts.
- Compact trend graphs through `sparkline`, accepting static or reactive values.
- Colour grids through `heatmap`.
- Key/value displays through `kv_grid`, with reactive labels and values.
- `markdown` renders headings, links, code, tables, and read-only task lists. `markdown_edit` provides editing with a preview.
- `html` renders a subset of HTML.
- `mermaid` renders diagrams from static or reactive source text.
- `diff` renders highlighted unified or split views, with virtualized rows and hunk or line selection.
- `code_edit` provides a code editor with syntax highlighting.
- Images through `image`, with PNG and JPEG decoding through `image_bytes`.
- Vector paths through `path` and custom drawing through `widgets::canvas`, using the same drawing commands on both hosts.
- Drag and drop through `drag_source`, `drop_target`, and `drag_ghost`, plus desktop file-drop handlers.

#### Styling and themes

- Typed styling through `Paint` and `StateVariants`, including typography, borders, shadows, and paint for hover, focus, press, and disabled states.
- Light, dark, and console themes through `Theme`, with shared metrics, semantic colours through `Token`, and colour families through `Tone`.
- Theme switching and TOML theme files through `parse_theme`, `set_theme`, `watch_theme`, and `AppConfig::theme_file`, including reload when the file changes.
- Right-to-left layout, locale-aware numbers and dates, and translated widget labels through theme direction, `style::Locale`, and `style::Strings`.

#### Text

- Static and reactive text through `text`, accepting strings, signals, or closures, with optional single-line rendering.
- Text shaping, bidirectional text, and IME composition for text inputs.
- Selectable text through `NodeEventsExt::selectable_text`, with clipboard-result callbacks through `on_selection_copy`.
- Syntax highlighting through `highlight` and `hl_spans` for Rust, shell, TOML, JSON, Python, JavaScript, TypeScript, Go, C/C++, YAML, CSS, HTML, SQL, Markdown, and diffs.

#### Animation

- Spring, tween, and keyframe animation through `animation_runtime::Animator`, `animation_runtime::Playback`, and `animation::Keyframes`.
- Style transitions and animated appearance or disappearance through `transition` and `presence`.
- Continuous motion through `animation_runtime::clock`, a reactive elapsed-time clock that advances on painted frames.
- Reduced-motion preferences through `animation_runtime::MotionPrefs`, with OS defaults and an `AppConfig::reduced_motion` override.

#### Window host

- Desktop windows on Linux, macOS, and Windows, and a browser canvas on WebAssembly through `AppConfig::canvas`.
- GPU rendering through wgpu and software rendering through the CPU backend. `SIGIL_BACKEND` selects automatic fallback, GPU, or CPU rendering.
- Multiple windows through `app::open_window`, sharing one event loop.
- Exit cleanup through `AppConfig::on_exit`, including window closure and operating-system quit or shutdown.

#### Terminal host

- The same widget tree on a character grid through `AppConfig::terminal`, with a configurable frame interval.
- Terminal capability detection through `terminal::caps`, including colour depth, background brightness, keyboard protocols, and glyph fallbacks.
- Images through Kitty, iTerm2, and Sixel protocols, with a glyph fallback.
- Keyboard input, bracketed and multi-line paste, mouse clicks and drags, and OSC 52 clipboard support.
- Inline mode through `AppConfig::inline`, preserving scrollback. `terminal::print_above` writes output above the live region.
- External event integration through `AppConfig::terminal_hooks`, and temporary suspension through `terminal::suspend`.

#### Accessibility

- AccessKit screen-reader support with roles, names, focus, values, live regions, and accessible document links. `Host::accessibility_tree` exposes the tree to embedded applications.
- Screen-reader announcements through `kernel::announce` and `kernel::announce_assertive`.
- Terminal screen-reader support with focus tracking, announcements, and an accessibility transcript.

#### Platform features

- Async file pickers and alerts through `app::dialog`. The `dialog` feature provides in-app dialogs on both hosts; `native-dialog` adds operating-system dialogs to the window host.
- Native menus through `app::menu`, with shortcut accelerators and menu events.
- System tray icons through `app::tray`, with updates through `app::tray::update`.
- Desktop notifications through `app::notify`.
- Global hotkeys through `app::hotkey`.
- Now-playing information and media-key events through `app::media`.
- Audio playback through `app::Player`, with queued tracks, volume control, and reactive playback state.
- Pseudo-terminal child processes through the optional `pty` feature.
- Text and image clipboard access through `clipboard::copy`, `clipboard::paste`, and optional image support.
- Files and URLs opened with the operating-system handler through `app::open`.

#### Tooling

- An optional inspector through the `inspect` feature and `SIGIL_INSPECT`, with accessibility queries, screenshots, input control, and frame profiles.
- Headless interaction testing through `sigil_kittest::Harness`, locating controls by accessible label and running async work as frames advance.
- Text and PNG snapshot comparisons through `Harness::snapshot`.
- macOS app bundles and Windows app directories through `sigil_sign::macos_app` and `sigil_sign::windows_dir`. `sigil_sign::CodeSign` signs macOS bundles and standalone executables; bundles can include a provisioning profile.
- Update-package signatures through `sign_file` and `verify_file`, binding the file name and expected version. `sigil-pack` provides packaging, signing, verification, and key generation.
- Optional code hot reload, including `Host::mount_hot` for embedded applications, preserving state created outside the root component and restoring focus.

#### Fonts and icons

- Bundled Fira Sans, Cascadia Mono, and a Noto Color Emoji subset in `sigil-fonts` under SIL Open Font License 1.1. The DejaVu Sans symbol subset uses the Bitstream Vera licence; the crate licence is OFL-1.1 AND Bitstream-Vera.
- Lucide icons through `Icon` and `icon`, tinted by the active text colour.
