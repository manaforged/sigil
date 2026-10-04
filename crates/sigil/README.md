# sigil-ui

UI in Rust. One widget tree. Open it in a window with `sigil::run`,
embed it with `sigil::Host`, or paint it on a character grid with
`sigil::run(AppConfig::new(title).terminal(), tree)` (feature `terminal`).

The package is `sigil-ui` because `sigil` is taken on crates.io. The
library name is `sigil`, so imports read `sigil::`.

## Status

0.1 is an early release. The API will break between minor versions until
1.0. The markdown, HTML, SVG, and mermaid parsers are not hardened for
untrusted input; they have been fuzzed only briefly. Report bugs on
[GitHub issues](https://github.com/manaforged/sigil/issues).

## Install

Follow [Install](https://github.com/manaforged/sigil#install). It also
gives the terminal-only line without the GPU stack.

Rust 1.90 or newer. The window host uses wgpu by default;
`SIGIL_BACKEND=cpu` selects the CPU renderer.

## Usage

Write `use sigil::prelude::*;`. The prelude brings in `run`, the
widgets, the signals, and `task`.
Widget helpers implement `IntoNode`; type a helper as `impl IntoNode`.
The second argument to `run` is a function that returns the app's root: `run("Go", || button("Go"))`.

A window:

```rust,no_run
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
```

The same tree in a terminal:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("hello-terminal").terminal(), || {
        text("hello")
    })
}
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
| `render` | no | `sigil::terminal::render` for scene output without a window. Turns on `scene`. |
| `capture` | no | Offscreen capture of a scene to pixels. Turns on `scene`. |
| `clipboard` | via `desktop` | `OsClipboard`, the system clipboard. |
| `clipboard-image` | no | `sigil::clipboard::image()`, images from the system clipboard. |
| `image-decode` | no | `image_bytes`, and `data:` images in `markdown`. |
| `dialog` | no | `sigil::app::dialog`: native open, save, folder, message, and confirm dialogs. Turns on `desktop`. |
| `menu` | no | `sigil::app::menu`: native application menus. Turns on `desktop`. |
| `tray` | no | `sigil::app::tray`: system tray icons and events. Turns on `menu`. |
| `audio` | no | `Player` for audio playback. |
| `hot` | no | Code hot reload under `dx serve --hot-patch`: `run` mounts the root again after each patch; an embed host uses `Host::mount_hot`. Uses `subsecond` and `dioxus-devtools`. Not on wasm. |
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

Styling is typed: paint through `Paint` methods and state variants,
layout through `LayoutStyle`. Typed
actions use a plain `impl Action for ...`; there is no derive macro.

## Docs

The [user guide](https://github.com/manaforged/sigil/tree/main/docs/guide)
covers each widget, the terminal host, and troubleshooting. The
[API map](https://github.com/manaforged/sigil/blob/main/docs/api.md)
maps each job to its symbol.

License: MIT. The default fonts ship in the separate `sigil-fonts` crate,
which the `scene` feature pulls in, under the SIL Open Font License 1.1.

## API

<!-- truesight:surface sigil-ui -->

3304 public items (444 types, 251 functions, 1238 methods, 520 fields, 790 variants, 61 constants) at 3814 paths; 510 are re-export aliases. 1825 trait impl lines, 206 inherent impl lines. 36 modules, 1 re-export only.

| Module | Items | Types | Functions | Methods | Aliases |
| --- | ---: | ---: | ---: | ---: | ---: |
| `sigil` | 60 | 4 | 2 | 53 | 0 |
| `sigil::a11y` | 13 | 4 | 3 | 5 | 0 |
| `sigil::animation` | 76 | 9 | 0 | 33 | 0 |
| `sigil::animation_runtime` | 52 | 8 | 8 | 23 | 0 |
| `sigil::animation_runtime::presets` | 8 | 0 | 0 | 0 | 0 |
| `sigil::app` | 114 | 23 | 15 | 11 | 0 |
| `sigil::app::dialog` | 15 | 2 | 5 | 8 | 0 |
| `sigil::app::menu` | 3 | 0 | 3 | 0 | 0 |
| `sigil::app::tray` | 2 | 0 | 2 | 0 | 0 |
| `sigil::bidi` | 11 | 2 | 2 | 4 | 0 |
| `sigil::clipboard` | 16 | 5 | 5 | 4 | 0 |
| `sigil::events` | 193 | 26 | 4 | 66 | 0 |
| `sigil::geometry` | 39 | 7 | 0 | 17 | 0 |
| `sigil::hal` | 62 | 24 | 0 | 25 | 0 |
| `sigil::icons` | 12 | 3 | 0 | 2 | 0 |
| `sigil::icons::builtin` | 9 | 0 | 0 | 0 | 0 |
| `sigil::kernel` | 209 | 23 | 12 | 112 | 0 |
| `sigil::layout` | 225 | 30 | 5 | 93 | 0 |
| `sigil::prelude` (re-export only) | 0 | 0 | 0 | 0 | 510 |
| `sigil::profile` | 213 | 13 | 19 | 13 | 0 |
| `sigil::reactive` | 48 | 9 | 11 | 24 | 0 |
| `sigil::reactive::debug` | 2 | 0 | 2 | 0 | 0 |
| `sigil::renderer` | 56 | 9 | 1 | 20 | 0 |
| `sigil::renderer::capture` | 6 | 1 | 2 | 0 | 0 |
| `sigil::scene` | 251 | 36 | 3 | 52 | 0 |
| `sigil::shell` | 25 | 5 | 0 | 15 | 0 |
| `sigil::style` | 586 | 61 | 7 | 174 | 0 |
| `sigil::style::palette` | 19 | 1 | 0 | 12 | 0 |
| `sigil::style::scale` | 27 | 2 | 14 | 3 | 0 |
| `sigil::style::variant` | 12 | 3 | 0 | 1 | 0 |
| `sigil::tasks` | 50 | 9 | 10 | 22 | 0 |
| `sigil::terminal` | 176 | 27 | 34 | 35 | 0 |
| `sigil::text` | 33 | 5 | 2 | 21 | 0 |
| `sigil::widgets` | 654 | 90 | 80 | 366 | 0 |
| `sigil::widgets::interaction` | 11 | 1 | 0 | 10 | 0 |
| `sigil::widgets::layout_ext` | 16 | 2 | 0 | 14 | 0 |

<!-- /truesight -->

