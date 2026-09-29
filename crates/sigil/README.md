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

```toml
[dependencies]
sigil-ui = "0.1"
```

[Install](https://github.com/manaforged/sigil#install) also gives the
terminal-only line without the GPU stack.

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
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().gap(16.0))
            .class("p-32 bg-surface")
            .child(text(move || format!("Count: {}", count.get())))
            .child(button("Increment").primary().activate(move || count.update(|n| *n += 1))),
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

`.class("...")` parses the class string at runtime. An unknown class
panics in a debug build with the nearest valid names, and logs one
warning in a release build. Typed
actions use a plain `impl Action for ...`; there is no derive macro.

## Docs

The [user guide](https://github.com/manaforged/sigil/tree/main/docs/guide)
covers each widget, the terminal host, and troubleshooting. The
[API map](https://github.com/manaforged/sigil/blob/main/docs/api.md)
maps each job to its symbol.

License: MIT. The bundled fonts under `assets/text/fonts` are under the
SIL Open Font License; see `LICENSE-OFL.txt` there.

## API

<!-- truesight:surface sigil-ui -->

3197 public items (420 types, 216 functions, 1317 methods, 477 fields, 711 variants, 56 constants) at 5975 paths; 2778 are re-export aliases. 1700 trait impl lines, 192 inherent impl lines. 90 modules, 20 re-export only.

| Module | Items | Types | Functions | Methods | Aliases |
| --- | ---: | ---: | ---: | ---: | ---: |
| `sigil` | 101 | 12 | 12 | 53 | 26 |
| `sigil::a11y` (re-export only) | 0 | 0 | 0 | 0 | 7 |
| `sigil::a11y::bridge` | 9 | 3 | 0 | 5 | 0 |
| `sigil::a11y::mapping` | 3 | 1 | 2 | 0 | 0 |
| `sigil::a11y::snapshot` | 1 | 0 | 1 | 0 | 1 |
| `sigil::animation` | 76 | 9 | 0 | 33 | 0 |
| `sigil::animation_runtime` | 36 | 5 | 6 | 18 | 26 |
| `sigil::animation_runtime::binding_animate` | 16 | 3 | 2 | 5 | 0 |
| `sigil::animation_runtime::presets` | 8 | 0 | 0 | 0 | 0 |
| `sigil::app` | 35 | 10 | 6 | 2 | 76 |
| `sigil::app::open` | 1 | 0 | 1 | 0 | 1 |
| `sigil::app::profile` (re-export only) | 0 | 0 | 0 | 0 | 146 |
| `sigil::bidi` | 11 | 2 | 2 | 4 | 0 |
| `sigil::clipboard` | 14 | 5 | 4 | 3 | 0 |
| `sigil::events` | 52 | 5 | 6 | 37 | 0 |
| `sigil::events::prelude` (re-export only) | 0 | 0 | 0 | 0 | 143 |
| `sigil::hal` | 60 | 7 | 0 | 14 | 181 |
| `sigil::hal::a11y` | 7 | 5 | 0 | 2 | 0 |
| `sigil::hal::events` | 80 | 14 | 0 | 13 | 40 |
| `sigil::hal::geometry` | 26 | 4 | 0 | 8 | 0 |
| `sigil::hal::layout` | 11 | 2 | 0 | 4 | 0 |
| `sigil::hal::renderer` | 7 | 4 | 0 | 3 | 0 |
| `sigil::hal::scheduler` | 7 | 4 | 0 | 3 | 0 |
| `sigil::hal::shell` | 40 | 8 | 0 | 10 | 0 |
| `sigil::hal::size` | 11 | 2 | 0 | 6 | 0 |
| `sigil::hal::text` | 27 | 7 | 0 | 7 | 0 |
| `sigil::icons` | 12 | 3 | 0 | 2 | 0 |
| `sigil::icons::builtin` | 9 | 0 | 0 | 0 | 0 |
| `sigil::interaction` (re-export only) | 0 | 0 | 0 | 0 | 1 |
| `sigil::kernel` | 198 | 21 | 9 | 106 | 0 |
| `sigil::kernel::prelude` (re-export only) | 0 | 0 | 0 | 0 | 76 |
| `sigil::kernel::prelude::image` (re-export only) | 0 | 0 | 0 | 0 | 12 |
| `sigil::kernel::prelude::node` (re-export only) | 0 | 0 | 0 | 0 | 19 |
| `sigil::kernel::prelude::text` (re-export only) | 0 | 0 | 0 | 0 | 7 |
| `sigil::layout` | 205 | 26 | 3 | 79 | 8 |
| `sigil::layout::builder_ext` | 2 | 1 | 0 | 1 | 0 |
| `sigil::layout::prelude` (re-export only) | 0 | 0 | 0 | 0 | 93 |
| `sigil::layout::reactive_position` | 8 | 3 | 0 | 5 | 0 |
| `sigil::open` (re-export only) | 0 | 0 | 0 | 0 | 2 |
| `sigil::prelude` (re-export only) | 0 | 0 | 0 | 0 | 923 |
| `sigil::prelude::heatmap` (re-export only) | 0 | 0 | 0 | 0 | 3 |
| `sigil::prelude::interactive` (re-export only) | 0 | 0 | 0 | 0 | 3 |
| `sigil::prelude::number_field` (re-export only) | 0 | 0 | 0 | 0 | 7 |
| `sigil::prelude::open` (re-export only) | 0 | 0 | 0 | 0 | 2 |
| `sigil::prelude::patterns` (re-export only) | 0 | 0 | 0 | 0 | 43 |
| `sigil::profile` | 159 | 11 | 15 | 12 | 0 |
| `sigil::reactive` | 58 | 10 | 14 | 30 | 0 |
| `sigil::reactive::debug` | 2 | 0 | 2 | 0 | 0 |
| `sigil::renderer` | 55 | 9 | 1 | 19 | 10 |
| `sigil::renderer::capture` | 6 | 1 | 2 | 0 | 0 |
| `sigil::scene` | 251 | 36 | 3 | 52 | 0 |
| `sigil::shell` | 25 | 5 | 0 | 15 | 0 |
| `sigil::style` | 105 | 19 | 1 | 78 | 420 |
| `sigil::style::animator_overlay` | 25 | 3 | 0 | 5 | 0 |
| `sigil::style::atom` | 156 | 19 | 0 | 4 | 0 |
| `sigil::style::binding` | 19 | 3 | 0 | 16 | 0 |
| `sigil::style::builder_ext` | 18 | 2 | 1 | 12 | 0 |
| `sigil::style::engine` | 6 | 1 | 0 | 5 | 0 |
| `sigil::style::focus_ring` | 13 | 1 | 0 | 12 | 0 |
| `sigil::style::merge` | 6 | 1 | 5 | 0 | 0 |
| `sigil::style::metrics` | 25 | 2 | 1 | 13 | 0 |
| `sigil::style::overlay` | 10 | 1 | 0 | 9 | 0 |
| `sigil::style::palette` | 19 | 1 | 0 | 12 | 0 |
| `sigil::style::parser` | 13 | 2 | 2 | 0 | 0 |
| `sigil::style::prelude` (re-export only) | 0 | 0 | 0 | 0 | 331 |
| `sigil::style::reactive` | 2 | 1 | 0 | 1 | 0 |
| `sigil::style::resolve` | 2 | 0 | 2 | 0 | 0 |
| `sigil::style::scale` | 27 | 2 | 14 | 3 | 0 |
| `sigil::style::size` | 9 | 2 | 0 | 1 | 0 |
| `sigil::style::state` | 31 | 4 | 0 | 17 | 0 |
| `sigil::style::struct_api` | 79 | 1 | 0 | 78 | 0 |
| `sigil::style::styled` (re-export only) | 0 | 0 | 0 | 0 | 86 |
| `sigil::style::text_style` | 33 | 6 | 1 | 6 | 0 |
| `sigil::style::theme` | 56 | 3 | 1 | 30 | 4 |
| `sigil::style::token` | 110 | 2 | 0 | 11 | 0 |
| `sigil::style::tone` | 11 | 2 | 0 | 2 | 0 |
| `sigil::style::variant` | 12 | 3 | 0 | 1 | 0 |
| `sigil::style::variant_kit` | 3 | 0 | 0 | 0 | 0 |
| `sigil::tasks` | 47 | 9 | 11 | 19 | 0 |
| `sigil::text` | 33 | 5 | 2 | 21 | 20 |
| `sigil::text::bidi` (re-export only) | 0 | 0 | 0 | 0 | 7 |
| `sigil::widgets` | 636 | 79 | 68 | 373 | 20 |
| `sigil::widgets::heatmap` (re-export only) | 0 | 0 | 0 | 0 | 3 |
| `sigil::widgets::interaction` | 11 | 1 | 0 | 10 | 0 |
| `sigil::widgets::interactive` | 6 | 1 | 1 | 4 | 1 |
| `sigil::widgets::layout_ext` | 17 | 2 | 0 | 15 | 0 |
| `sigil::widgets::number_field` | 9 | 2 | 1 | 6 | 1 |
| `sigil::widgets::overlay` | 0 | 0 | 0 | 0 | 0 |
| `sigil::widgets::overlay::heatmap` | 2 | 0 | 1 | 0 | 1 |
| `sigil::widgets::patterns` | 17 | 2 | 13 | 2 | 28 |

<!-- /truesight -->

