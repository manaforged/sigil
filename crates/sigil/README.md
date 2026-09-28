# sigil-ui

UI in Rust. One widget tree. Open it in a window with `sigil::run`,
embed it with `sigil::Host`, or paint it on a character grid with
`sigil::run(AppConfig::new(title).terminal(), tree)` (feature `terminal`).

The package is `sigil-ui` because `sigil` is taken on crates.io. The
library name is `sigil`, so imports read `sigil::`.

## Install

```bash
cargo add sigil-ui
```

For the terminal host only, without the GPU stack:

```bash
cargo add sigil-ui --no-default-features --features terminal
```

Rust 1.90 or newer. The window host uses wgpu by default;
`SIGIL_BACKEND=cpu` selects the CPU renderer.

## Usage

Write `use sigil::prelude::*;`. The prelude brings in `run`, `Host`,
`root`, the widgets, the signals, and `copy`, `paste`, and `install`.
Widget helpers implement `IntoNode`; type a helper as `impl IntoNode`.
Pass a single widget to `run` with `root(button("Go"))`.

A window:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 240.0).gap(16.0))
            .class(classes!("p-8 bg-surface"))
            .child(text(label).class(classes!("text-on-surface")))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

The same tree in a terminal:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("hello-terminal").terminal(), |cx: &mut Cx| {
        cx.add(text("hello"))
    })
}
```

An embed host. You own the window and the GPU:

```rust
use sigil::prelude::*;

Runtime::scope("embed", || {
    let mut host = Host::new();
    host.mount(text("hello"));
    let scene = host.scene((80.0, 24.0)).expect("layout");
    assert!(scene.find_text("hello").is_some());
    drop(host.surfaces());
});
```

`Host::scene` lays out the tree and returns the `CommandScene` that the
window renderer and the terminal rasteriser both draw. `Host::emit`
returns the scene for the current layout. After layout,
`Host::surfaces` lists the `surface(key)` holes. You place a native
view in each hole; Sigil does not create an OS window for it.

Clipboard: call `copy("text")` inside `run`, or after
`install(OsClipboard)` on an embed host.

With the `desktop` feature, `open(url)` passes an `http`, `https`, or
`mailto` URL to the OS handler. It rejects every other scheme.

## Features

| Feature | Default | Enables |
| --- | --- | --- |
| `desktop` | yes | The window host for `run`, `open`, and the OS clipboard. Turns on `scene`, `clipboard`, and winit with AccessKit. |
| `terminal` | no | The terminal host: `run` with `.terminal()`, and `sigil::terminal`. Uses crossterm. |
| `scene` | via `desktop` | The wgpu renderer, the CPU renderer, text shaping, SVG icons, and `sigil::text`. No window. |
| `cpu` | via `scene` | The CPU renderer that `SIGIL_BACKEND=cpu` selects. |
| `render` | no | `sigil::render` for scene output without a window. Turns on `scene`. |
| `capture` | no | Offscreen capture of a scene to pixels. Turns on `scene`. |
| `clipboard` | via `desktop` | `OsClipboard`, the system clipboard. |
| `clipboard-image` | no | `sigil::clipboard::image()`, images from the system clipboard. |
| `image-decode` | no | `image_bytes`, and `data:` images in `markdown`. |
| `dialog` | no | `sigil::dialog`: native open, save, folder, message, and confirm dialogs. Turns on `desktop`. |
| `menu` | no | `sigil::menu`: native application menus. Turns on `desktop`. |
| `tray` | no | `sigil::tray`: system tray icons and events. Turns on `menu`. |
| `audio` | no | `Player` for audio playback. |
| `hot` | no | Code hot reload under `dx serve --hot-patch`: `run` mounts the root again after each patch, and `Host::mount_hot`. Uses `subsecond` and `dioxus-devtools`. Not on wasm. |
| `pty` | no | `Session`: a child process on a pseudo-terminal. |
| `kittest` | no | Test support for `sigil-kittest`. Not API. |

## Limits

- `signal`, `derived`, and `effect` panic outside `Runtime::scope`.
  `run` opens that scope. An embed host that uses signals must call
  `Runtime::scope`.
- `run` does not place a native view in a `surface(key)` hole.
- `html()` parses a document subset. No CSS, no JavaScript, no browser.
- `markdown`, `html`, `mermaid`, and the SVG parser reject sources
  larger than 1 MiB.
- A `pty` session opens at 24x80 and does not resize.
- Public modules not listed in `docs/api.md` are used by tests and
  macro expansion. They are not API and can change in any 0.1.x release.

`classes!` joins its string literals and parses the class string at
runtime. An unknown class logs a warning and has no effect. Typed
actions use a plain `impl Action for ...`; there is no derive macro.

## Docs

The [user guide](https://github.com/manaforged/sigil/tree/main/docs/guide)
covers each widget, the terminal host, and troubleshooting. The
[API map](https://github.com/manaforged/sigil/blob/main/docs/api.md)
maps each job to its symbol.

License: MIT. The bundled fonts under `assets/text/fonts` are under the
SIL Open Font License; see `LICENSE-OFL.txt` there.
