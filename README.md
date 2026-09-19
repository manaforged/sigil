# Sigil

Desktop UI in Rust. One widget tree runs on three hosts: `launch` opens
a window, `Host` embeds the tree in another engine's loop, and the
separate `sigil-tui` crate paints it on a character grid.

![The workbench example application](docs/workbench.png)

Unreleased. The API can change between 0.x releases.

## Requirements

- Rust 1.90 or newer. This checkout pins Rust 1.98.1 in `rust-toolchain.toml`.
- A wgpu-capable GPU for the default desktop renderer.
  Set `SIGIL_BACKEND=cpu` to use the CPU renderer. The terminal host needs no GPU.

Hosted Linux CI uses Mesa software Vulkan for renderer checks. macOS and
Windows require separate platform checks.

## Install

Sigil is not published on crates.io. The repository is currently private;
you need GitHub access and Git authentication to use this dependency.

```bash
cargo add sigil-gui --git https://github.com/manaforged/sigil --branch main
```

The package is `sigil-gui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`. The first
build compiles the wgpu stack; it can take a few minutes. `Cargo.lock`
records the selected Git commit.

For a terminal-only program:

```bash
cargo add sigil-tui --git https://github.com/manaforged/sigil --branch main
cargo add sigil-gui --git https://github.com/manaforged/sigil --branch main --no-default-features
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
`sigil_tui::run`. Run these examples from a clone of this repository:

```bash
cargo run -p sigil-gui --example hello-counter   # the app above
cargo run -p sigil-gui --example hello-host      # embed the tree
cargo run -p sigil-tui --example hello-tui      # terminal host
mdbook serve docs/book                          # the guide and architecture
```

## Limits

- Headless machines run the terminal host and the test suite. The
  windowed hosts need a GPU adapter, unless `SIGIL_BACKEND=cpu` selects
  the CPU renderer. `auto` is the default; `wgpu` requires an adapter.
- No supported web or mobile host. A wasm renderer proof exists but is
  not in this repository.
- The terminal host has no IME, no rounded corners, and no shadows.
  Images render through the terminal graphics protocol on Kitty,
  Ghostty, WezTerm, Konsole, iTerm2, and Warp. Other terminals show
  half-block cells.
- No fuzz targets. Do not feed untrusted documents to the markdown,
  HTML, or SVG parsers.

## Security

Report vulnerabilities privately to [thomas@manaforge.tech](mailto:thomas@manaforge.tech).
GitHub private reporting is also an option when it is available on the
repository. See [SECURITY.md](SECURITY.md).
The workspace denies `unsafe`, with one exception in the native menu
path (`crates/sigil/src/app/menu.rs`). `cargo deny check` gates
advisories.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the build and the gate.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
