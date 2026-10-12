# Sigil

A GUI and TUI framework written in Rust. One widget tree runs in a window, in a terminal, or inside another engine's loop.

- **One tree, three hosts**: `sigil::run` opens a window, or paints a character grid with the `terminal` feature, and `Host` embeds the tree in another engine.
- **GPU or CPU**: a wgpu renderer, and a CPU renderer for machines without a GPU.
- **Web too**: the same app builds for `wasm32`, drawing with WebGPU or into a 2D canvas.
- **Batteries included**: 67 components, including markdown, tables, trees, charts, mermaid diagrams, and a code editor.
- **Accessible**: the window host exposes the widget tree to screen readers through AccessKit.

[Book](docs/SUMMARY.md) | [API map](docs/api.md) | [Examples](crates/sigil/examples) | [Guides](docs/guide)

![The workbench example application](docs/workbench.png)

Status: unreleased. The API will break between minor versions until 1.0. The markdown, HTML, SVG, and mermaid parsers are not hardened for untrusted input; fuzz targets cover them, but the runs so far are short. Report bugs on [GitHub issues](https://github.com/manaforged/sigil/issues).

## Requirements

- Rust 1.90 or newer.
- On Linux, the window host needs the GTK3, udev, ALSA, xkbcommon, and xdo development packages. On Debian and Ubuntu: `sudo apt-get install libgtk-3-dev libudev-dev libasound2-dev libxkbcommon-dev libxkbcommon-x11-dev libxdo-dev`. The terminal host needs none of them.
- A wgpu-capable GPU for the default renderer of the window host. Set `SIGIL_BACKEND=cpu` to use the CPU renderer. The terminal host needs no GPU.

## Platforms

| Host | Linux | macOS | Windows | Web (`wasm32`) |
| --- | --- | --- | --- | --- |
| Window `run` | yes (wgpu or `SIGIL_BACKEND=cpu`) | yes | yes | yes (WebGPU, or CPU into a 2D canvas) |
| Embed `Host` | yes | yes | yes | yes |
| Terminal `run` (feature `terminal`) | yes | yes | yes | no |

No mobile host.

## Install

Sigil is not on crates.io yet. Add the Git dependency:

```toml
[dependencies]
sigil-ui = { git = "https://github.com/manaforged/sigil" }
```

The package is `sigil-ui` because the `sigil` name is taken on crates.io. The library is `sigil`, so imports read `sigil::`. The first build compiles the wgpu stack and can take a few minutes.

For a terminal-only program:

```toml
[dependencies]
sigil-ui = { git = "https://github.com/manaforged/sigil", default-features = false, features = ["terminal"] }
```

To pack release builds, install the `sigil-sign` package. It provides the `sigil-pack` command.

```bash
cargo install --git https://github.com/manaforged/sigil sigil-sign
```

To sign packages from your own code, add `sigil-sign = { git = "https://github.com/manaforged/sigil" }`; see [Signing](docs/guide/signing.md).

## Usage

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

`sigil::run` opens the window and drives the frame loop. Pass `AppConfig::new(title).terminal()` to run the same tree in the terminal. The embed host pumps `Host` inside `Runtime::scope`. To run the examples, clone this repository and run:

```bash
cargo run -p sigil-ui --example hello-counter                        # the app above
cargo run -p sigil-ui --example hello-host                           # embed the tree
cargo run -p sigil-ui --example hello-terminal --features terminal   # terminal host
```

## Cargo features

The [`sigil-ui` README](crates/sigil/README.md#features) lists every feature. `desktop` is the default. `terminal` adds the terminal host.

## Limits

- Headless machines run the terminal host and the test suite. The window host needs a GPU adapter, unless `SIGIL_BACKEND=cpu` selects the CPU renderer. `auto` is the default; `wgpu` requires an adapter.
- No mobile host. The web host has no system fonts, no clipboard paste, and no accessibility tree, and it runs tasks on the page's one thread; see [Web host](docs/guide/web.md).
- The terminal host has no rounded corners and no shadows. It composes CJK input through the terminal's own input method. Images render through the terminal graphics protocol on Kitty, Ghostty, WezTerm, Konsole, iTerm2, Warp, and Sixel terminals. Other terminals show sextant glyphs.
- Six fuzz targets cover the markdown, HTML, SVG, mermaid, class-string and scene-render inputs. Read [SECURITY.md](SECURITY.md) before you pass them untrusted input.
- `sigil-pack macos` ad-hoc codesigns unless you pass a signing identity. Ad-hoc signing is not Developer ID or notarization.

## Docs

| Read | For |
| --- | --- |
| [User guide](docs/guide/README.md) | Task pages in reading order. Build the book with `mdbook serve docs`. |
| [Troubleshooting](docs/guide/troubleshooting.md) | No GPU, images in tmux, and terminal colour. |
| [GUI and TUI](docs/guide/hosts.md) | Build for the window, the terminal, or both; host-only APIs; how to split host code. |
| [Recipes](docs/guide/recipes.md) | Keys, theme switching, layout, streaming, timers, terminal hooks, and screen readers. |
| [API map](docs/api.md) | Every exported item with its signature and an example, in one Markdown file. |
| [Architecture](docs/guide/arch/overview.md) | How it works inside. |

The [changelog](CHANGELOG.md) lists each release.

## Security

Report vulnerabilities by email to [thomas@manaforge.tech](mailto:thomas@manaforge.tech). [SECURITY.md](SECURITY.md) lists the other reporting channel and each input surface.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue first, and send a patch only when asked. See [CONTRIBUTING.md](CONTRIBUTING.md) for the build and the gate.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.

The bundled fonts and icons keep their own licenses: see [the font notice](crates/sigil-fonts/fonts/NOTICE.md) (SIL OFL 1.1, Bitstream Vera) and [the icon notice](crates/sigil/assets/icons/NOTICE.md) (Lucide, ISC and MIT).
