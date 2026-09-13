# sigil-gui

GPU UI in Rust. One widget tree. Open it with `sigil::launch`, embed it
with `sigil::Host`, or paint it on a character grid with the separate
`sigil-tui` crate (`sigil_tui::run`).

The package is `sigil-gui` because `sigil` is taken on crates.io. The
library name is `sigil`, so imports read `sigil::`.

Write `use sigil::prelude::*;`. The prelude brings in `launch`, `Host`,
`Painted`, `root`, widgets, and `copy`/`paste`/`install` with the
`clipboard` feature. `Player` needs the `audio` feature; `Session`
needs `pty`. Widget helpers implement `IntoNode`; type a helper as
`impl IntoNode`. Pass another widget to `launch` with
`root(button("Go"))`. The terminal host is the separate `sigil-tui`
crate, entered with `sigil_tui::run`.

## Requirements

Rust 1.90. A GPU for `launch`. A terminal for the `sigil-tui` crate.

## Install

```bash
cargo add sigil-gui
```

For the terminal host:

```bash
cargo add sigil-tui
cargo add sigil-gui --no-default-features
```

## Usage

Desktop window:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label_text = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 240.0).gap(16.0))
            .class(classes!("p-8 bg-surface"))
            .child(text(label_text).class(classes!("text-on-surface")))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

Embed host. You own the window and the GPU:

```rust
use sigil::prelude::*;

Runtime::scope("embed", || {
    let mut host = Host::new();
    host.mount(text("hello"));
    assert!(host.layout((80.0, 24.0)).expect("layout").is_some());
    assert!(
        host.painted()
            .iter()
            .any(|node| node.text.as_deref() == Some("hello"))
    );
    let _holes = host.surfaces();
});
```

`Host::scene` needs feature `desktop` or `render`. Without it, use
`Host::painted`. After layout, `Host::surfaces` lists `surface(key)`
holes. You place a native view; Sigil does not create an OS window for
the hole.

Clipboard: `copy("text")` after `launch`, or after
`install(MyClipboard)` on an embed host.

With the `desktop` feature, open a URL or path with the OS handler:
`open("https://example.com")`.

## Limits

- `signal` / `derived` / `effect` panic outside `Runtime::scope`.
  `launch` opens that scope. An embed host that uses signals must call
  `Runtime::scope`.
- `Host::scene` needs `desktop` or `render`. `render` does not pull
  winit.
- `launch` does not parent a native view into a `surface(key)` hole.
- `html()` parses a document subset. No CSS, no JavaScript, no browser.
- `markdown`, `html`, `mermaid`, and the SVG baker reject sources
  larger than 1 MiB.
- A `pty` session opens at 24x80. There is no resize path yet.
- Modules marked `#[doc(hidden)]` are used by tests and macro expansion.
  They are not API and can change in any 0.1.x release.
- The crate is `0.1.0`. Names can still move.

## Features

| Feature | Adds |
|---|---|
| `desktop` (default) | `launch`, OS chrome, `Host::scene` |
| `clipboard` | system clipboard access through `sigil::clipboard` |
| `clipboard-image` | `sigil::clipboard::image()`, native clipboard pixels |
| `dialog` | native open, save, and folder dialogs |
| `menu` | native application menus |
| `tray` | system tray icons and events |
| `render` | `Host::scene`, `sigil::render`, `sigil::text`. No winit |
| `audio` | `Player::play_path`. The player holds the stream; `tick` writes progress; `seek` jumps |
| `pty` | `pty::Session::spawn` |
| `image-decode` | `image_bytes` |
| `test-support` | `dialog::mock` |

## Docs

- Book: `mdbook serve docs/book`
- Examples: `cargo run -p sigil-gui --example hello-counter` and
  `cargo run -p sigil-gui --example hello-host`. The larger apps live in
  the `sigil-examples` checkout.

## Macro note

`classes!` concatenates its string literal arguments and parses them at
runtime through `sigil::style::parse_class_string`. The parser caches
the atoms. Typed actions use a plain `impl Action for ...`; there is no
derive macro.

License: MIT. The bundled fonts under `assets/text/fonts` are under the
SIL Open Font License; see `LICENSE-OFL.txt` there.
