# Sigil

Desktop UI in Rust. One widget tree, three hosts: `launch` opens a
window, `Host` embeds the tree in another engine's loop, and `run_tui`
paints it on a character grid. The widget catalog (button, table,
text_field, virtual_list, modal, dropdown, tabs, chart, …) lives in
`sigil::widgets`.

![The workbench example application](site/assets/screenshots/workbench.png)

**Status:** alpha (`0.1.0-alpha.1`). The API is not stable; pin exact
versions.

## Install

The alpha is not on crates.io yet. From this repository:

```bash
cargo add --git https://github.com/ThomGardiner/sigil.git sigil-ui
```

The package is named `sigil-ui` because `sigil` was already taken on
crates.io. The library name is `sigil`, so application imports start with
`sigil::`. After the first crates.io publish, `cargo add sigil-ui@0.1.0-alpha.1`
is the install.

## Requirements

- Rust 1.90 or newer (`rust-toolchain.toml` pins stable for development)
- macOS, Windows, or Linux
- A wgpu-capable GPU for the windowed hosts; the TUI host needs none

## Run

```bash
cargo run -p hello-counter        # smallest runnable app
cargo run -p hello-host           # embed the tree in your own loop
cargo run -p widget-playground    # live widget catalog
mdbook serve docs/book            # author guide and architecture
```

## Usage

Put this code in `src/main.rs`, then run `cargo run`:

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

The repository includes runnable examples for events, styles, tables, and
headless rendering under [`examples/`](examples/).

## Run in a terminal

Select the terminal host instead of the default desktop host:

```bash
cargo add --git https://github.com/ThomGardiner/sigil.git sigil-ui \
  --no-default-features --features tui
```

Call `sigil::run_tui` with the same tree. See the
[terminal host guide](docs/book/06-terminal.md) for a complete example and
the differences between desktop and terminal rendering.

## How it works

No virtual DOM and no per-frame rebuild. A signal write marks the
nodes it touched, and a clean frame skips layout and scene assembly.
Frames run a typed phase machine (Effect, Layout, Paint); a write
during paint panics in debug builds and defers in release.

- **Layout, text, and rendering are owned.** Flexbox, block, and grid
  layout (Taffy stays a bench corpus); HarfRust shaping, skrifa
  metrics, bidi, and font fallback; wgpu rendering with glyphs and SVG
  icons in one GPU pipeline.
- **Accessibility is a node property.** AccessKit, IME, drag-and-drop,
  and animation wire from the same tree.

The architecture chapters are in [`docs/book/arch/`](docs/book/arch/);
the crate-by-crate map is
[crates.md](docs/book/arch/crates.md).

## Benchmarks

Two stress workloads ship as parity builds against reference
implementations, and the site's Performance page documents the
measurement rules: same host, checked-in seed and viewport, an
output-equivalence assertion, frame-time distributions.

```bash
cargo run --release -p access-log    # 1,000,000-row live tape, 60 x 8 cells @ 16 ms
cargo run --release -p data-table    # 10,000 rows x 24 columns, parity with GPUI 0.2.2
```

No speed numbers are published without that record. Measurement
scripts live in `scripts/`.

## Limits

- Alpha: the API changes between releases. Pin exact versions.
- Headless machines can run the TUI host and the test suite, but not
  the windowed hosts: rendering needs an adapter.
- Web and mobile hosts are not included.
- The terminal host does not support IME, images, rounded corners, or shadows.
- Fuzz targets do not exist yet. Do not feed untrusted documents to
  the markdown, HTML, or SVG parsers (see Security).
- Public interaction-test helpers are limited; see the
  [testing guide](docs/book/04-testing.md) for the current boundary.

## Security

Report vulnerabilities through GitHub's private vulnerability
reporting (Security tab), never as public issues. The workspace denies
`unsafe`, and `cargo deny check` gates dependency advisories. The
untrusted-input surfaces are the style-class parser, the markdown and
HTML ingest, text shaping, and SVG baking.

## Contributing

Issues are welcome. Unsolicited pull requests are not accepted; if a
patch is wanted, it will be asked for.

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
