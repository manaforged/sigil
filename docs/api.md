# Sigil API map

This page is for a coding agent or a tool that writes an app that
depends on Sigil. It maps each job to the symbol that does it.

Sigil is unreleased. The API can change between 0.x releases. Pin a git
**rev**. Do not track `main`.

The supported import surface is `sigil::prelude::*`, the items that
`sigil` re-exports at the crate root, the crate-root re-exports of
`sigil_tui`, and the crate-root items of `sigil_sign`. Modules marked
`#[doc(hidden)]` are public in the type system but are not API. Do not
import from them. That list includes `sigil::app`, `sigil::kernel`,
`sigil::layout`, `sigil::reactive`, `sigil::style`, `sigil::widgets`,
`sigil::events`, `sigil::hal`, `sigil::renderer`, `sigil::shell`, and
`sigil::text`. `Entity`, `EntityCx`, `AsyncContext`, and `Task` are
hidden and carry no stability promise.

## Core types

| Type | Role |
| --- | --- |
| `Cx` | Build context. The tree function receives `&mut Cx` and calls `cx.add(...)`. |
| `NodeId` | Handle of a node in the tree. The tree function returns the root `NodeId`. |
| `NodeBuilder` | Builder that `container()` and the widget functions return. |
| `Signal<T>` | Reactive value. Create it with `signal(value)`. |
| `Runtime` | Reactive runtime. `Runtime::scope(name, f)` runs `f` inside a runtime. |
| `AppConfig` | Window configuration. `(title, width, height)` converts into it. |
| `TreeRoot` | Trait of a value that builds the tree. A `fn(&mut Cx) -> NodeId` qualifies. |
| `Host` | Embed host. It builds, routes events to, lays out, and paints one tree. |
| `Painted` | One painted node that `Host::painted` returns. |
| `LayoutStyle` | Layout description, for example `LayoutStyle::flex_column()`. |
| `Theme` | Theme values. `parse_theme` and `set_theme` apply one on the desktop host. |
| `Error`, `AppResult<T>` | Application error and result. |

## Quick paths

Open a window:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));

    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 240.0).gap(16.0))
            .child(text(label))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

Embed the tree in another loop:

```rust,no_run
use sigil::prelude::*;

fn main() {
    Runtime::scope("hello-host", || {
        let mut host = Host::new();
        host.build(|cx| cx.add(container().child(text("Hello"))));
        if let Err(err) = host.layout((480.0, 240.0)) {
            eprintln!("{err}");
            return;
        }
        for node in host.painted() {
            println!("{:?}", node.text);
        }
    });
}
```

For a full program, copy `crates/sigil/examples/hello-counter.rs`,
`crates/sigil/examples/hello-host.rs`, or
`crates/tui/examples/hello-tui.rs`. Do not write a new application
shell first.

## Jobs to symbols

| Job | Symbol |
| --- | --- |
| Import the application surface | `use sigil::prelude::*;` |
| Open a window and run the frame loop | `sigil::launch(config, tree)` (feature `desktop`) |
| Open a file or URL with the operating system handler | `sigil::open(target)` (feature `desktop`) |
| Stop the application | `sigil::request_exit` |
| Send a window command | `request_window_command`, `WindowCommand` |
| Embed the tree in another engine | `Runtime::scope`, `Host::new`, `Host::build` or `Host::mount` |
| Route an input event to an embedded tree | `Host::route(&mut EventRouter, ShellEvent)` |
| Lay out and paint an embedded tree | `Host::layout`, `Host::painted`, `Host::paint_into`, `Host::repaint` |
| Get the rectangles of host-drawn surfaces | `surface(key)` in the tree, then `Host::surfaces` |
| Get the accessibility tree of an embedded tree | `Host::accessibility_tree` |
| Run the tree in a terminal | `sigil_tui::run(config, tree)` |
| Render one terminal frame to a string | `sigil_tui::present(tree, cols, rows, events)` |
| Drive the terminal host one step at a time | `sigil_tui::step`, `Host::cells` |
| Detect the terminal graphics protocol | `sigil_tui::detect`, `sigil_tui::Protocol` |
| Create state | `signal` |
| Compute a value from state | `derived`, `derived_eq` |
| Run a side effect when state changes | `effect`, `on_cleanup` |
| Group writes into one update | `batch` |
| Send events between parts of the app | `EventChannel` |
| Run async work and write the result to a signal | `spawn`, `signal_writer`, `SignalWriter`, `Status` |
| Post work to the owner thread | `post`, `dispatcher`, `dispatch_to_owner`, `OwnerHandle` |
| Add a box that holds children | `container`, `fragment` |
| Show text | `text`, `text_live`, `label` |
| Set layout | `.layout(LayoutStyle)`, `LayoutBuilderExt`, `WidgetLayoutExt` |
| Set style with utility classes | `.class(classes!("..."))` |
| Handle pointer and key events on a node | `NodeEventsExt`, `EventHandlers` |
| Move keyboard focus | `request_focus`, `FocusIntent` |
| Bind actions and shortcuts | `Action`, `HandlerKey`, `Propagate`, `Shortcut` |
| Buttons and toggles | `button`, `icon_button`, `checkbox`, `toggle`, `radio_group`, `chip` |
| Text and value input | `text_field`, `text_edit`, `number_field`, `date_field`, `color_field`, `slider`, `validated_field`, `field` |
| Code and documents | `code_edit`, `markdown`, `markdown_edit`, `html`, `mermaid`, `diff`, `terminal` |
| Choice lists | `dropdown`, `combobox`, `menu`, `menu_bar`, `command_menu` |
| Collections | `table`, `virtual_list`, `list_row`, `kv_grid`, `kv_row`, `tabs` |
| Application frame | `app_shell`, `sidebar`, `status_bar`, `panel`, `card`, `sheet`, `splitter`, `scroll`, `scroll_auto` |
| Overlays | `modal`, `tooltip`, `toast`, `toast_queue`, `hint_overlay`, `floating_text` |
| Data display | `chart`, `gauge`, `heatmap`, `progress`, `badge`, `dot`, `avatar` |
| Images, icons, and vector paths | `image`, `image_rgba`, `image_bytes` (feature `image-decode`), `icon`, `path` |
| Drag and drop | `DragDropExt`, `DragState`, `drag_ghost`, `axis_drag` |
| Animate a value | `Animator`, `AnimationConfig`, `Easing`, `Keyframes`, `SpringPreset`, `MotionPrefs` |
| Apply a theme | `parse_theme`, `set_theme` (feature `desktop`) |
| Set fonts and text tuning | `sigil::set_default_fonts`, `sigil::use_system_fonts`, `sigil::set_text_tuning` |
| Read and write the clipboard | `copy`, `paste`, `install`, `OsClipboard` (feature `clipboard`) |
| Native dialogs, menus, and tray | `sigil::dialog`, `sigil::menu`, `sigil::tray` (features `dialog`, `menu`, `tray`) |
| Play audio | `Player` (feature `audio`) |
| Run a terminal session | `Session` (feature `pty`) |
| Build a tree in a test | `sigil::testing::with_cx`, `sigil::testing::with_runtime`, `sigil::testing::tree_size` |
| Build a macOS `.app` | `sigil_sign::macos_app`, or `sigil-pack macos` |
| Build a Windows app directory | `sigil_sign::windows_dir`, or `sigil-pack windows` |
| Sign an update package with minisign | `sigil_sign::sign_file`, or `sigil-pack sign` |

The [widgets page](guide/widgets.md) describes each widget. The
[signing](guide/signing.md) and [packing](guide/packing.md) pages give
the `sigil-pack` arguments.

## Errors

| Type | Returned by | Meaning |
| --- | --- | --- |
| `sigil::Error` | `sigil::launch`, `sigil_tui::run` | Opaque application error. `Error::new` wraps any `std::error::Error`. `Error::source_error` returns the inner error. `LayoutError` converts into it. |
| `LayoutError` | `Host::layout`, `Host::scene`, `sigil_tui::present`, `sigil_tui::step` | The layout pass failed. |
| `sigil_sign::SignError::Key` | `sign_file` | The secret key could not be read or decrypted. |
| `sigil_sign::SignError::Package` | `sign_file` | The package could not be read. |
| `sigil_sign::SignError::Envelope` | `sign_file` | The signature could not be produced. |
| `sigil_sign::PackError::Missing` | `macos_app`, `windows_dir` | A required path is missing. |
| `sigil_sign::PackError::Io` | `macos_app`, `windows_dir` | A filesystem operation failed. |
| `sigil_sign::PackError::Codesign` | `macos_app` | `codesign` failed on macOS. |
| `sigil_sign::PackError::Escape` | `macos_app`, `windows_dir` | A path would land outside the bundle. |
| `sigil_sign::PackError::Symlink` | `macos_app`, `windows_dir` | The source tree contains a symlink or reparse point. |
| `sigil_sign::PackError::Occupied` | `macos_app`, `windows_dir` | The output path exists and is not a bundle that this crate wrote. |
| `sigil_sign::PackError::NotFile` | `macos_app`, `windows_dir` | The executable is not a regular file. |

`PanicPolicy::Isolate` is the default panic policy. `PanicPolicy::Abort`
is the other value.

## Conventions

- The package is `sigil-gui`. The library is `sigil`. Imports read
  `sigil::`.
- Application code imports `sigil::prelude::*`. Name a crate-root item
  with its path, for example `sigil::AppResult`.
- Make reactive writes on the thread that owns the runtime. From
  another thread, use `signal_writer`, `post`, or `dispatch_to_owner`.
- `sigil::launch` installs the OS clipboard. An embed host calls
  `install(OsClipboard)` itself.
- `SIGIL_BACKEND` accepts `auto`, `wgpu`, and `cpu`. `auto` is the
  default. `wgpu` requires a GPU adapter. Set `cpu` if there is no GPU.
- The default feature of `sigil-gui` is `desktop`. For a terminal-only
  program, add `sigil-gui` with `--no-default-features` and add
  `sigil-tui`. The README lists every Cargo feature.
- Do not feed untrusted markdown, HTML, or SVG to the parsers. The
  repository has no fuzz targets.
