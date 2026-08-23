# The application host

One tree: one signal graph, one node tree, one layout engine, one
command scene. Three ways to run it.

```text
                    UNIFIED CORE
     reactive · kernel · layout · style · events
     widgets · Host (build / route / layout / painted|scene)

     ┌──────────────┬──────────────┬──────────────┐
     │ Desktop app  │ Game / engine │ TUI          │
     │ launch       │ you pump Host│ run_tui      │
     │ winit + wgpu │ your GPU     │ cell raster  │
     └──────────────┴──────────────┴──────────────┘
```

| Mode | Entry | You own |
|---|---|---|
| Full application | `sigil::launch` | Nothing. Pumps `Host`. Window, GPU, OS clipboard, menu, tray, dialogs. |
| Embed | `sigil::Host` | The loop, the window, the GPU. Host emits `painted()` or `scene()`. |
| Terminal UI | `sigil::run_tui` | Nothing. Same tree on a character grid. |

`use sigil::prelude::*;` brings in `Host`, `launch`, `open`, `run_tui`,
`Player`, `Session`, `root`, `copy`, `paste`, and `set_clipboard`.
Widget helpers are `IntoNode`; type a helper as `impl IntoNode`.
Pass a widget to `launch` with `root(button("Go"))`.

`default-features = false` is core + widgets + `Host`. `desktop` and
`tui` are hosts. `render` adds `Host::scene` and the GPU scene
modules **without** winit. `audio` / `pty` are extra services.

## HAL vs services vs holes

Frame-loop traits live on `sigil-hal`: `Shell`, `Renderer`,
`TextEngine`, `A11yBridge`, `Scheduler`. `launch` binds winit and wgpu
and pumps `Host`. An embed host does **not** inject those traits into
`Host`. It reads `Host::painted` or `Host::scene` and draws with its
own GPU.

Clipboard, audio, PTY, and `open` are services, not HAL.

| Service | Speaks | Default | Embed |
|---|---|---|---|
| Clipboard | `Clipboard` + `set_clipboard` | OS on `launch` / `run_tui` | Call `set_clipboard` |
| Open | `open(target)` | `open` / `xdg-open` / `cmd start` | Call `open` |
| Audio | `Player` signals | rodio (`audio`) | Write the same signals |
| PTY | `Session.lines` | portable-pty (`pty`) | Mock session |
| Surface hole | `Host::surfaces()` | Skip GPU/TUI paint | Place a native view, or ignore |

`terminal()` and `transport()` stay in widgets. They compile without
rodio or a PTY.

## Surface holes

`surface(key)` is a layout box. Scene emit and TUI paint skip it and
its children. After layout, `Host::surfaces` returns `(key, rect)`.
Sigil does not create an OS window for that rect. Closing a spare
window must not quit the app, so `launch` leaves the hole empty. Fill
it from the embed host, or leave it empty.

## What this is not

- **Not a webview.** `html(source)` parses a document subset into the
  same nodes as `markdown`. No CSS, no JavaScript, no browser.
- **Not libghostty.** `run_tui` paints this tree onto cells.
  `terminal(lines)` is a PTY pane inside the tree.
- **Not AppKit as the UI.** Menu, dialog, and tray are OS chrome.
  Content is the GPU (or cell) tree. Style the app bar with
  `menu_bar().class(...)`. `set_menu_bar` is opt-in OS chrome (NSMenu /
  Win32) and applies on the next frame. `set_tray` is the same. Tray
  build failure logs `sigil::tray` and leaves no icon.

## Author path — desktop

```rust,ignore
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Notes", 640, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let doc = signal(String::from("<h1>Notes</h1><p>Hello</p>"));
    cx.add(
        container()
            .child(html(doc))
            .child(button("Copy").activate(|| {
                let _ = sigil::clipboard::copy("Hello");
            }))
            .child(surface(1)),
    )
}
```

Call `clipboard::copy` from a handler. The result is an [`AppResult`];
handle it. Do not drop it to hide a pasteboard failure.

## Author path — embed

```rust,ignore
use sigil::prelude::*;

fn pump() {
    Runtime::scope("game", || {
        let mut host = Host::new();
        host.build(app_tree);
        let _ = host.layout((800.0, 600.0));
        let holes = host.surfaces();
        let _ = holes;
        #[cfg(feature = "render")]
        {
            let _scene = host.scene((800.0, 600.0));
        }
    });
}
```

`signal()` panics outside `Runtime::scope`. `launch` and `run_tui`
open that scope. A `Host` that uses signals must open it. A static
tree does not. `host.mount(text("hello"))` is enough. See
`examples/hello-host`.

Feature `render` (or `desktop`) is required for `Host::scene`. Without
it, use `Host::painted`.

`launch` runs Effect → Layout → Paint. A host that writes during a
paint fence should do the same:

1. `Runtime::begin_effect()`
2. `host.route(...)`
3. `Runtime::begin_layout()`
4. `host.layout(viewport)`
5. `host.scene(viewport)` or `host.painted()`
6. `Runtime::begin_paint()`
7. Submit the GPU
8. `Runtime::end_frame()`

Tests that write in Idle can skip the phase calls. `Signal::set` runs
effects immediately outside a paint fence.

A game clipboard:

```rust,ignore
sigil::set_clipboard(MyPasteboard);
```

## Features

| Feature | What it adds |
|---|---|
| `desktop` (default) | `launch`, dialogs, menu, tray, OS clipboard, `Host::scene` |
| `tui` | `run_tui`, OS clipboard |
| `render` | `Host::scene`, `sigil::render`, `sigil::text`. No winit |
| `audio` | `Player` (`play_path`, `seek`) |
| `pty` | `Session` (`spawn`) |

Widgets for transport and terminal stay in the catalog either way.
`Player::play_path` holds the output stream on the player. `tick`
writes progress when the decoder reports a duration. `Player::seek`
jumps when duration is known. `launch` drives that tick. `Session::write` needs `Session::spawn`; paint is
`Session.lines`. `Session::spawn` needs `Runtime::scope` (the error
says so).
