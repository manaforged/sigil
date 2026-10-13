# sigil-ui

GUI and TUI framework in Rust. One widget tree. Open it in a window with `sigil::run`, embed it with `sigil::Host`, or paint it on a character grid with `sigil::run(AppConfig::new(title).terminal(), tree)` (feature `terminal`).

The package is `sigil-ui` because `sigil` is taken on crates.io. The library name is `sigil`, so imports read `sigil::`.

## Status

0.1 is the first release. The API will break between minor versions until 1.0. The markdown, HTML, SVG, and mermaid parsers are not hardened for untrusted input; fuzz targets cover them, but the runs so far are short. Report bugs on [GitHub issues](https://github.com/manaforged/sigil/issues).

## Install

Follow [Install](https://github.com/manaforged/sigil#install). It also gives the terminal-only line without the GPU stack.

Rust 1.90 or newer. The window host uses wgpu by default; `SIGIL_BACKEND=cpu` selects the CPU renderer.

## Usage

Write `use sigil::prelude::*;`. The prelude brings in `run`, the widgets, the signals, and `task`. Widget helpers implement `IntoNode`; type a helper as `impl IntoNode`. The second argument to `run` is a function that returns the app's root: `run("Go", || button("Go"))`.

A window:

```rust,no_run
# #[cfg(feature = "desktop")]
# mod window {
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - hello-counter").size(480, 240), app_tree)
}

fn app_tree() -> impl IntoNode {
    let count = signal(0_i32);
    column()
        .layout(LayoutStyle::default().padding(32.0))
        .bg(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
# }
# fn main() {}
```

The same tree in a terminal:

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal {
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("hello-terminal").terminal(), || {
        text("hello")
    })
}
# }
# fn main() {}
```

An embed host. You own the window and the GPU:

```rust
use sigil::Host;
use sigil::prelude::*;
use sigil::reactive::Runtime;

Runtime::scope("embed", || {
    let mut host = Host::new();
    host.mount(text("hello"));
    let scene = host.scene((80.0, 24.0)).expect("layout");
    assert!(scene.find_text("hello").is_some());
    drop(host.surfaces());
});
```

`Host::scene` lays out the tree and returns the `CommandScene` that the window renderer and the terminal rasterizer both draw. `Host::emit` returns the scene for the current layout. After layout, `Host::surfaces` lists the `surface(key)` holes. You place a native view in each hole; Sigil does not create an OS window for it.

Clipboard: call `copy("text")` inside `run`, or after `install(OsClipboard)` on an embed host.

With the `desktop` feature, `open(url)` passes an `http`, `https`, or `mailto` URL to the OS handler. It rejects every other scheme.

## Features

| Feature | Default | Enables |
| --- | --- | --- |
| `desktop` | yes | The window host for `run`, `open`, and the OS clipboard. Turns on `scene`, `clipboard`, and winit with AccessKit. |
| `terminal` | no | The terminal host: `run` with `.terminal()`, and `sigil::terminal`, including `sigil::terminal::render`. Uses crossterm and enables `image-decode`. |
| `scene` | via `desktop` | The wgpu renderer, the CPU renderer, offscreen capture through `sigil::renderer::capture`, text shaping, SVG icons, and `sigil::text`. No window. |
| `shell` | via `desktop` | The winit window shell with AccessKit. |
| `cpu` | via `scene` | The CPU renderer that `SIGIL_BACKEND=cpu` selects. |
| `shaping-complex` | via `scene` | Complex-script text shaping through `HarfRust`. |
| `render` | no | Scene rendering without a window. Turns on `scene`. |
| `clipboard` | via `desktop` | `OsClipboard`, the system clipboard. |
| `clipboard-image` | no | `sigil::clipboard::image()`, images from the system clipboard. |
| `image-decode` | no | `image_bytes`, and `data:` images in `markdown`. |
| `dialog` | no | `sigil::app::dialog`: in-app open, save, folder, message, and confirm dialogs on either host. |
| `native-dialog` | no | Native dialogs on the window host; the terminal host keeps the in-app dialogs. Turns on `dialog` and `desktop`. |
| `menu` | no | `sigil::app::menu`: native application menus. Turns on `desktop`. |
| `tray` | no | `sigil::app::tray`: system tray icons and events. Turns on `menu`. |
| `audio` | no | `Player` for audio playback. |
| `notify` | no | Desktop notifications through `sigil::app::notify`. |
| `media` | no | Now-playing metadata and media-key events through `sigil::app::media`. Turns on `desktop`. |
| `hotkey` | no | Global keyboard shortcuts through `sigil::app::hotkey`. Turns on `desktop`. |
| `hot` | no | Code hot reload under `dx serve --hot-patch`: `run` mounts the root again after each patch; an embed host uses `Host::mount_hot`. Uses `subsecond` and `dioxus-devtools`. Not on wasm. |
| `pty` | no | `Session`: a child process on a pseudo-terminal. |
| `kittest` | no | Test support for `sigil-kittest`. Not API. |
| `test-clock` | no | Manual interval clock for tests. |
| `inspect` | no | The local inspection socket when `SIGIL_INSPECT` is set. |

## Limits

- `signal`, `derived`, and `effect` panic outside `Runtime::scope`. `run` opens that scope. An embed host that uses signals must call `Runtime::scope`.
- `run` does not place a native view in a `surface(key)` hole.
- `html()` parses a document subset. No CSS, no JavaScript, no browser.
- `markdown`, `html`, `mermaid`, and the SVG parser reject sources larger than 1 MiB.
- A `pty` session opens at 24x80 and does not resize.
- Public modules not listed in `docs/api.md` are used by tests and macro expansion. They are not API and can change in any 0.1.x release.

Styling is typed: paint through `Paint` methods and state variants, layout through `LayoutStyle`. Typed actions use a plain `impl Action for ...`; there is no derive macro.

## Docs

The [user guide](https://github.com/manaforged/sigil/tree/main/docs/guide) covers each widget, the terminal host, and troubleshooting. The [API map](https://github.com/manaforged/sigil/blob/main/docs/api.md) maps each job to its symbol.

License: MIT. The default fonts ship in the separate `sigil-fonts` crate, which the `scene` feature pulls in, under the SIL Open Font License 1.1 and, for the symbol font, the Bitstream Vera license.
