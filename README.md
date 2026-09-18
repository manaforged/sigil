# Sigil

Desktop UI in Rust. One widget tree runs on three hosts: `launch` opens
a window, `Host` embeds the tree in another engine's loop, and the
separate `sigil-tui` crate paints it on a character grid.

![The workbench example application](docs/workbench.png)

Version 0.1.0. The API can change between 0.x releases.

## Requirements

- Rust 1.90 or newer. `rust-toolchain.toml` pins 1.98.1 for development;
  CI compile-checks the 1.90 floor.
- A wgpu-capable GPU for the windowed hosts. The terminal host needs none.

| Platform | CI | GPU tier |
| --- | --- | --- |
| macOS | local gate | runs before a release |
| Windows | local gate | runs before a release |
| Linux | full CI | hosted runners have no adapter |

## Install

Until 0.1.0 is on crates.io, depend on the repository:

```toml
[dependencies]
sigil-gui = { git = "https://github.com/manaforged/sigil", branch = "main" }
```

The package is `sigil-gui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`. The first
build compiles the wgpu stack; it can take a few minutes.

For a terminal-only program:

```toml
[dependencies]
sigil-tui = { git = "https://github.com/manaforged/sigil", branch = "main" }
sigil-gui = { git = "https://github.com/manaforged/sigil", branch = "main", default-features = false }
```

## Usage

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - hello-counter", 480, 240), app_tree)
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

`sigil::launch` opens the window and drives the frame loop. An embed
host pumps `Host` inside `Runtime::scope`. The terminal entry point is
`sigil_tui::run`. Runnable examples ship with the crates:

```bash
cargo run -p sigil-gui --example hello-counter   # the app above
cargo run -p sigil-gui --example hello-host      # embed the tree
cargo run -p sigil-tui --example hello-tui      # terminal host
mdbook serve docs/book                          # the guide and architecture
```

The larger apps live in the `sigil-examples` checkout.

## Limits

- Headless machines run the terminal host and the test suite. The
  windowed hosts need a GPU adapter, unless `SIGIL_BACKEND=cpu` selects
  the CPU renderer. `auto` is the default; `wgpu` requires an adapter.
- No supported web or mobile host. The wasm renderer proof lives in
  `sigil-examples`.
- The terminal host has no IME, no rounded corners, and no shadows.
  Images render through the terminal graphics protocol on Kitty,
  Ghostty, WezTerm, Konsole, iTerm2, and Warp. Other terminals show
  half-block cells.
- No fuzz targets. Do not feed untrusted documents to the markdown,
  HTML, or SVG parsers.

## Security

Report a vulnerability through GitHub's private reporting on this
repository, never as a public issue. See [SECURITY.md](SECURITY.md).
The workspace denies `unsafe`, with one exception in the native menu
path (`crates/sigil/src/app/menu.rs`). `cargo deny check` gates
advisories.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the build and the gate.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
