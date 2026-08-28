# Sigil

Desktop UI in Rust. One widget tree, three hosts: `launch` opens a
window, `Host` embeds the tree in another engine's loop, and `run_tui`
paints it on a character grid. The widget catalog (button, table,
text_field, virtual_list, modal, dropdown, tabs, chart, …) lives in
`sigil::widgets`.

**Status:** alpha (`0.1.0-alpha.1`). The API is not stable; pin exact
versions.

## Requirements

- Rust 1.90 (pinned in `rust-toolchain.toml`)
- A wgpu-capable GPU for the windowed hosts; the TUI host needs none

## Run

```bash
cargo run -p hello-counter        # smallest runnable app
cargo run -p widget-playground    # live widget catalog
mdbook serve docs/book            # author guide and architecture
```

## Usage

```rust
use sigil::prelude::*;

fn app(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().gap(8.0))
            .child(text(derived(move || count.get().to_string())))
            .child(button("+").activate(move || count.set(count.peek() + 1))),
    )
}
```

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

## Limits

- Alpha: the API changes between releases. Pin exact versions.
- Headless machines can run the TUI host and the test suite, but not
  the windowed hosts: rendering needs an adapter.
- Fuzz targets do not exist yet. Do not feed untrusted documents to
  the markdown, HTML, or SVG parsers (see Security).

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
