# Sigil

Desktop UI in Rust. One widget tree runs on three hosts. `sigil::run`
opens a window or, with the `terminal` feature, paints the tree on a
character grid. `Host` embeds the tree in another engine's loop.

![The workbench example application](docs/workbench.png)

Using a coding agent with Sigil? Give it
[docs/api.md](https://raw.githubusercontent.com/manaforged/sigil/main/docs/api.md).

The API can change between 0.x releases. Depend on one minor version.

## Requirements

- Rust 1.90 or newer.
- A wgpu-capable GPU for the default desktop renderer.
  Set `SIGIL_BACKEND=cpu` to use the CPU renderer. The terminal host needs no GPU.

## Platforms

| Host | Linux | macOS | Windows | Web (`wasm32`) |
| --- | --- | --- | --- | --- |
| Window `run` | yes (wgpu or `SIGIL_BACKEND=cpu`) | yes | yes | yes (WebGPU, or CPU into a 2D canvas) |
| Embed `Host` | yes | yes | yes | yes |
| Terminal `run` (feature `terminal`) | yes | yes | yes | no |

No mobile host.

## Install

```bash
cargo add sigil-ui
```

The package is `sigil-ui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`. The first
build compiles the wgpu stack and can take a few minutes.

For a terminal-only program:

```bash
cargo add sigil-ui --no-default-features --features terminal
```

To pack and sign release builds, add `sigil-sign`:

```bash
cargo add sigil-sign
```

## Usage

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

`sigil::run` opens the window and drives the frame loop. Pass
`AppConfig::new(title).terminal()` to run the same tree in the terminal.
An embed host pumps `Host` inside `Runtime::scope`. To run the
examples, clone this repository and run:

```bash
cargo run -p sigil-ui --example hello-counter                        # the app above
cargo run -p sigil-ui --example hello-host                           # embed the tree
cargo run -p sigil-ui --example hello-terminal --features terminal   # terminal host
```

## Cargo features

The [`sigil-ui` README](crates/sigil/README.md#features) lists every
feature. `desktop` is the default. `terminal` adds the terminal host.

## Limits

- Headless machines run the terminal host and the test suite. The
  window host needs a GPU adapter, unless `SIGIL_BACKEND=cpu` selects
  the CPU renderer. `auto` is the default; `wgpu` requires an adapter.
- No mobile host. The web host has no IME, no system fonts, no
  clipboard paste, no task executor, and no accessibility tree; see
  [Web host](docs/guide/web.md).
- The terminal host has no IME, no rounded corners, and no shadows.
  Images render through the terminal graphics protocol on Kitty,
  Ghostty, WezTerm, Konsole, iTerm2, Warp, and Sixel terminals. Other
  terminals show sextant glyphs.
- Fuzz targets are in progress. Read [SECURITY.md](SECURITY.md) before
  you pass untrusted documents to the markdown, HTML, or SVG parsers.
- `sigil-pack macos` ad-hoc codesigns unless you pass a signing
  identity. Ad-hoc signing is not Developer ID or notarization.
- A form can show a doubled label after save and reopen (issue #5).

## Docs

| Read | For |
| --- | --- |
| [User guide](docs/guide/README.md) | Task pages in reading order. Build the book with `mdbook serve docs`. |
| [Troubleshooting](docs/guide/troubleshooting.md) | No GPU, images in tmux, and terminal colour. |
| [Recipes](docs/guide/recipes.md) | Keys, theme switching, layout, streaming, timers, terminal hooks, and screen readers. |
| [API map](docs/api.md) | Every exported item with its signature and an example. Written for agents and tools too. |
| [Architecture](docs/guide/arch/overview.md) | How it works inside. |

The [changelog](CHANGELOG.md) lists each release.

## Security

Report vulnerabilities by email to [thomas@manaforge.tech](mailto:thomas@manaforge.tech).
[SECURITY.md](SECURITY.md) lists the other reporting channel and each
input surface.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the build and the gate.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
