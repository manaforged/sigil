# Sigil for coding agents

Point your coding agent at this file when it builds an app with Sigil. It covers the dependency line, where the API is listed, and the patterns that the examples use.

Sigil is a Rust UI library. One widget tree runs in a window, in a terminal, or embedded in another engine's loop. The package is `sigil-ui` and the library is `sigil`.

## Add the dependency

Sigil is not on crates.io yet.

```toml
[dependencies]
sigil-ui = { git = "https://github.com/manaforged/sigil" }
```

For a terminal-only program:

```toml
[dependencies]
sigil-ui = { git = "https://github.com/manaforged/sigil", default-features = false, features = ["terminal"] }
```

For tests:

```toml
[dev-dependencies]
sigil-kittest = { git = "https://github.com/manaforged/sigil" }
```

Rust 1.90 or newer. The window host on Linux needs the GTK3, udev, ALSA, xkbcommon, and xdo development packages.

## Find the API

Read these before you write a helper. Most jobs already have a function.

| File | Use it to |
| --- | --- |
| [docs/api.md](https://raw.githubusercontent.com/manaforged/sigil/main/docs/api.md) | Look up every item at the crate root and in the prelude, with its signature, purpose, and an example that compiles. The "Jobs to symbols" section maps a job to the call. |
| [api.txt](https://raw.githubusercontent.com/manaforged/sigil/main/docs/reference/sigil-ui/api.txt) | Search every public item by path, one per line. |
| [API reference index](https://raw.githubusercontent.com/manaforged/sigil/main/docs/reference/sigil-ui/index.md) | Index of the per-module reference pages and the examples. |
| [Recipes](https://raw.githubusercontent.com/manaforged/sigil/main/docs/guide/recipes.md) | Keys, theme switching, layout, streaming, timers, host hooks, and screen readers. |
| [GUI and TUI](https://raw.githubusercontent.com/manaforged/sigil/main/docs/guide/hosts.md) | Build for the window, the terminal, or both. |
| [Testing](https://raw.githubusercontent.com/manaforged/sigil/main/docs/guide/testing.md) | Drive the UI from a test with `sigil_kittest::Harness`. |
| [Troubleshooting](https://raw.githubusercontent.com/manaforged/sigil/main/docs/guide/troubleshooting.md) | No GPU, images in tmux, and terminal color. |

## Write the app

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Counter"), app)
}

fn app() -> impl IntoNode {
    let count = signal(0_i32);
    column()
        .bg(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
```

- Import `sigil::prelude::*`. Reach into a module path only for an item that the prelude does not export.
- `sigil::run` takes an `AppConfig` and a component. `AppConfig::new(title).terminal()` runs the same tree in the terminal.
- State lives in signals. `signal(value)` creates one. `get` inside a closure subscribes that closure, `peek` reads without subscribing, and `set` and `update` write.
- Text that changes takes a closure: `text(move || ...)`. A plain string never updates.
- `show(cond, then, otherwise)` switches between subtrees. `for_each(items, key, render)` renders a keyed list.
- Colors come from the theme: pass a `Token` such as `Token::Surface` or `Token::Accent` to `bg`, `text_color`, or `border_color`. Do not hard-code colors.
- Async work uses `task` (reruns and cancels) or `spawn` (runs once). Inside one, `tasks::blocking` runs blocking work and `tasks::sleep` waits.
- Write signals on the UI thread. From another thread, use `signal_writer`, or `tasks::on_message` for a channel.
- Public event, command, error, and config types are `#[non_exhaustive]`. Match them with a `_` arm, and build `AppConfig` with its builder methods.

## Test the app

```rust
use sigil::prelude::*;
use sigil_kittest::Harness;

fn counter() -> impl IntoNode {
    let count = signal(0);
    row()
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("+").activate(move || count.update(|n| *n += 1)))
}

Harness::new(counter).click("+").expect_text("Count: 1");
```

The harness finds controls through the accessibility tree, runs frames until the UI settles, and runs on the terminal host by default. It needs no GPU.

## Run the examples

Clone this repository to run them. Each one is a small, complete app.

```bash
cargo run -p sigil-ui --example story-02-counter
cargo run -p sigil-ui --example story-04-todo
cargo run -p sigil-ui --example story-06-async
cargo run -p sigil-ui --example hello-terminal --features terminal
cargo run -p sigil-ui --example chat-both --features terminal,desktop
```

## Limits

- The API can break between 0.x releases.
- The markdown, HTML, SVG, and mermaid parsers are not hardened for untrusted input. Read [SECURITY.md](https://raw.githubusercontent.com/manaforged/sigil/main/SECURITY.md) before you feed them user content.
- The window host needs a GPU adapter unless `SIGIL_BACKEND=cpu` selects the CPU renderer. The terminal host needs none.
- The terminal host has no rounded corners, no shadows, no menus, and no tray.
