# Sigil

Desktop UI in Rust. One widget tree runs on three hosts: `launch` opens
a window, `Host` embeds the tree in another engine's loop, and the
separate `sigil-tui` crate paints it on a character grid.

![The workbench example application](docs/workbench.png)

Using a coding agent with Sigil? Give it
[docs/api.md](https://raw.githubusercontent.com/manaforged/sigil/main/docs/api.md).

Unreleased. The API can change between 0.x releases. Pin a git **rev**,
not `main`, from a product until `v0.1.0` exists.

## Requirements

- Rust 1.90 or newer. This checkout pins Rust 1.98.1 in `rust-toolchain.toml`.
- A wgpu-capable GPU for the default desktop renderer.
  Set `SIGIL_BACKEND=cpu` to use the CPU renderer. The terminal host needs no GPU.

## Platforms

| Host | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Desktop `launch` | yes (wgpu or `SIGIL_BACKEND=cpu`) | yes | yes |
| Embed `Host` | yes | yes | yes |
| Terminal `sigil-tui` | yes | yes | yes |
| Web / mobile | no | no | no |

## Install

Sigil is not published on crates.io. The repository is currently private;
you need GitHub access and Git authentication to use this dependency.

```bash
cargo add sigil-gui --git https://github.com/manaforged/sigil --rev <sha>
cargo add sigil-sign --git https://github.com/manaforged/sigil --rev <sha>
```

Replace `<sha>` with a commit from this repository. The package is
`sigil-gui` because the `sigil` name is taken on crates.io. The library
is `sigil`, so imports read `sigil::`. The first build compiles the wgpu
stack; it can take a few minutes. `Cargo.lock` records the selected
commit.

For a terminal-only program:

```bash
cargo add sigil-tui --git https://github.com/manaforged/sigil --rev <sha>
cargo add sigil-gui --git https://github.com/manaforged/sigil --rev <sha> --no-default-features
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
```

## Cargo features

| Feature | Default | Role |
| --- | --- | --- |
| `desktop` | yes | Windowed host (`launch`) |
| `scene` | via desktop | Layout, text, widgets, renderer |
| `shell` | via desktop | winit + AccessKit |
| `cpu` | no | `SIGIL_BACKEND=cpu` software renderer |
| `clipboard` | via desktop | OS clipboard |
| `dialog` / `menu` / `tray` | no | Native dialogs, menus, tray |
| `audio` / `pty` | no | Playback and terminal sessions |
| `image-decode` | no | Image file decode |
| `render` | no | Scene without a window |

The table lists the features of `sigil-gui`. `sigil-tui` is a separate
crate. `sigil-sign` packs and signs.

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
- Intermittent doubled label after save and reopen in a form (issue #5).
- `sigil-pack macos` ad-hoc codesigns. That is not Developer ID or
  notarization.

## Docs

| Read | For |
| --- | --- |
| [User guide](docs/guide/README.md) | Task pages in reading order. Build the book with `mdbook serve docs`. |
| [API map](docs/api.md) | For agents and tools: each job mapped to its symbol or command, the error model, and what is off-limits. |
| [Architecture](docs/guide/arch/overview.md) | How it works inside. |

The [changelog](CHANGELOG.md) states the version policy.

## Security

Report vulnerabilities privately to [thomas@manaforge.tech](mailto:thomas@manaforge.tech).
See [SECURITY.md](SECURITY.md) for the other reporting channel.
The workspace denies `unsafe`, with one exception in the native menu
path (`crates/sigil/src/app/menu.rs`). `cargo deny check` gates
advisories.

## Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the build and the gate.

## License

[MIT](LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
