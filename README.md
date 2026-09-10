# Sigil

Desktop UI in Rust. One widget tree, three hosts: `launch` opens a
window, `Host` embeds the tree in another engine's loop, and `run_tui`
paints it on a character grid. The widgets (button, table, text_field,
virtual_list, modal, dropdown, tabs, chart) live in `sigil::widgets`.

![The workbench example application](site/assets/screenshots/workbench.png)

Version 0.1.0. The API moves between 0.x releases; pin an exact rev.

## Requirements

- Rust 1.90 or newer. `rust-toolchain.toml` selects stable.
- macOS, Windows, or Linux.
- A wgpu-capable GPU for the windowed hosts. The terminal host needs none.

## Install

The crates are not on crates.io. Add the facade from this repository:

```bash
cargo add --git https://github.com/ThomGardiner/sigil.git sigil-ui
```

The package is `sigil-ui` because `sigil` was taken on crates.io. The
library is still `sigil`, so imports read `sigil::`. For a terminal-only
binary add `--no-default-features --features tui`.

## Usage

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - counter", 480, 240), app_tree)
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

Swap `sigil::launch` for `sigil::run_tui` and the same tree paints in a
terminal. Runnable examples:

```bash
cargo run -p hello-counter        # the app above
cargo run -p hello-host           # embed the tree in your own loop
cargo run -p widget-playground    # every widget in one window
mdbook serve docs/book            # the guide and the architecture chapters
```

## How it works

A signal write marks the nodes it touched. A clean frame skips layout
and scene assembly; there is no virtual DOM and no per-frame rebuild.
Frames run three phases (Effect, Layout, Paint). A write during Paint
panics in a debug build and is deferred in release.

Layout (flexbox, block, grid), text shaping (HarfRust, skrifa, bidi,
font fallback), and wgpu rendering are owned code in this workspace.
AccessKit, IME, drag-and-drop, and animation attach to the same tree.
The crate map is [crates.md](docs/book/arch/crates.md).

## Benchmarks

Two workloads ship as parity builds against reference implementations.
Each run records the host, a checked-in seed and viewport, and an
output-equivalence assertion before any frame time is reported.

```bash
cargo run --release -p access-log --bin access-log-sigil   # 1,000,000-row live tape, 60 x 8 cells, 16 ms tick
cargo run --release -p data-table                          # 10,000 rows x 24 columns, the GPUI 0.2.2 example ported 1:1
```

## Limits

- Headless machines run the terminal host and the test suite. The
  windowed hosts need a GPU adapter.
- No supported web or mobile host. `examples/web-demo` is a wasm
  renderer proof.
- The terminal host has no IME, images, rounded corners, or shadows.
- No fuzz targets yet. Do not feed untrusted documents to the markdown,
  HTML, or SVG parsers.
- The public interaction-test helpers are `with_cx`, `with_runtime`,
  and `tree_size`; see the [testing guide](docs/book/04-testing.md).

## Security

Report a vulnerability through GitHub's private reporting on this
repository, never as a public issue. See [SECURITY.md](SECURITY.md).
The workspace denies `unsafe` except one waived Win32 call that installs
the native menu (`crates/app/src/menu.rs`). `cargo deny check` gates
advisories.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked.

## Influences

[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui),
[Taffy](https://github.com/DioxusLabs/taffy),
[SolidJS](https://www.solidjs.com/),
[Leptos](https://github.com/leptos-rs/leptos), Eric Lengyel's
[Slug](https://jcgt.org/published/0006/02/02/), and the W3C
[Flexbox](https://www.w3.org/TR/css-flexbox-1/) and
[Grid](https://www.w3.org/TR/css-grid-2/) specifications.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
