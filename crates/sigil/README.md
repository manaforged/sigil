# sigil-ui

GPU UI in Rust. One widget tree. Open it with `sigil::launch`, pump
it with `sigil::Host`, or paint it on a terminal with `sigil::run_tui`.

The package is `sigil-ui` because `sigil` was taken on crates.io. The
library name is `sigil`, so imports read `sigil::…`.

Write `use sigil::prelude::*;`. That brings in `launch`, `Host`,
`Painted`, `Player`, `Session`, `root`, `run_tui`, widgets,
`copy`, `paste`, and `set_clipboard`. Widget helpers are `IntoNode`;
type a helper as `impl IntoNode`. Pass other widgets to `launch`
with `root(button("Go"))`. `launch` and `run_tui` pump `Host`.

## Requirements

Rust 1.90. A GPU for `launch`. A terminal for `run_tui`.

## Install

The alpha is not on crates.io yet. From this repository:

```bash
cargo add --git https://github.com/ThomGardiner/sigil.git sigil-ui
```

For the terminal host, disable the desktop defaults and enable `tui`:

```bash
cargo add --git https://github.com/ThomGardiner/sigil.git sigil-ui \
  --no-default-features --features tui
```

## Usage

Desktop window (`examples/hello-counter`):

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

Embed host. You own the window and the GPU (`examples/hello-host`):

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
holes. You place a native view. Sigil does not create an OS window
for the hole.

Clipboard: `copy("text")` after `launch` / `run_tui`,
or after `set_clipboard` on an embed host.

Open a URL or path with the OS handler: `open("https://example.com")`.

## Limits

- `signal` / `derived` / `effect` panic outside `Runtime::scope`.
  `launch` and `run_tui` open that scope. An embed host that uses
  signals must call `Runtime::scope`.
- `Host::scene` needs `desktop` or `render`. `render` does not pull
  winit.
- `launch` does not parent a native view into a `surface(key)` hole.
- `html()` parses a document subset. No CSS, no JavaScript, no browser.
- The crate is `0.1.0-alpha.1`. Names can still move.

## Features

| Feature | Adds |
|---|---|
| `desktop` (default) | `launch`, OS chrome, `Host::scene` |
| `tui` | `run_tui` |
| `clipboard` | system clipboard access through `sigil::clipboard` |
| `dialog` | native open, save, and folder dialogs |
| `menu` | native application menus |
| `tray` | system tray icons and events |
| `render` | `Host::scene`, `sigil::render`, `sigil::text`. No winit |
| `audio` | `Player::play_path` (holds the stream; `tick` writes progress; `seek` jumps) |
| `pty` | `pty::Session::spawn` |
| `image-decode` | `image_bytes` |
| `test-support` | `dialog::mock` |

## Docs

- Book: `mdbook serve docs/book`
- Examples: `cargo run -p hello-counter`, `cargo run -p hello-host`,
  `cargo run -p hello-tui`

## Macro note

`classes!` and `#[derive(Action)]` expand to `sigil::__private::…`.
Depend on this facade crate. Keep the package name `sigil-ui`.

License: MIT.
