# Sigil for coding agents

Point your coding agent at this file when it builds an app with Sigil. It covers the dependency line, where the API is listed, and the patterns that the examples use.

Sigil is a Rust UI library. One widget tree runs in a window, in a terminal, or embedded in another engine's loop. The package is `sigil-ui` and the library is `sigil`.

## Add the dependency

```toml
[dependencies]
sigil-ui = "0.2"
```

For a terminal-only program:

```toml
[dependencies]
sigil-ui = { version = "0.2", default-features = false, features = ["terminal"] }
```

For tests:

```toml
[dev-dependencies]
sigil-kittest = "0.2"
```

Rust 1.96 or newer. The window host on Linux needs the GTK3, udev, ALSA, xkbcommon, and xdo development packages.

## Find the API

Read these before you write a helper. Most jobs already have a function.

| File | Use it to |
| --- | --- |
| [docs.rs/sigil-ui](https://docs.rs/sigil-ui) | Look up every public item, with its signature and a compiled example on each main type. |
| `cargo doc -p sigil-ui --open` | Browse every public item, generated from the source. |
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
        .background(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
```

- Import `sigil::prelude::*`. Reach into a module path only for an item that the prelude does not export.
- `sigil::run` takes an `AppConfig` and a component. `AppConfig::new(title).terminal()` runs the same tree in the terminal.
- State lives in signals. `signal(value)` creates one. `get` inside a closure subscribes that closure, `peek` reads without subscribing, and `set` and `update` write.
- Text that changes takes a closure: `text(move || ...)`. A plain string never updates.
- `show(cond, then, otherwise)` switches between subtrees. `switch(key).case(v, || ...).otherwise(|| ...)` picks among keyed branches. `for_each(items, key, render)` renders a keyed list; `items` is a signal, a `memo`, or a closure.
- Add children with `.child(node)` or `.children(iter)`; every widget builder implements `Into<NodeBuilder>`.
- Colors come from the theme: pass a `Token` such as `Token::Surface` or `Token::Accent` to `background`, `text_color`, or `border_color`. A signal or closure of a token also works: `.background(move || if on.get() { Token::Accent } else { Token::Surface })`. Do not hard-code colors.
- `.theme(theme)` restyles the colors and corners of one subtree. `Theme::for_backdrop(backdrop).token(Token::Custom("brand"), color)` adds a custom token.
- `.label(text)` on `checkbox`, `toggle`, `gauge`, and `progress` shows visible text that is also the accessible name. `.accessible(name)` sets only the name that a screen reader announces, on any widget, such as an icon button.
- `dropdown`, `segmented`, and `radio_group` take a `Signal<usize>` of the selected index and a list of option labels.
- `command_menu(open)` shows a command palette while the `Signal<bool>` `open` is true. Choosing an item, Escape at the top level, or a backdrop press sets it to false.
- Async work uses `task` (reruns and cancels) or `spawn` (runs once). A `task` waits for `.run()`; `task(load).autostart()` starts it when it is created. Inside one, `tasks::blocking` runs blocking work and `tasks::sleep` waits.
- Write signals on the UI thread. From another thread, use `Signal::writer`, or `tasks::listen(rx, on_message)` for a tokio or `std::sync::mpsc` channel. Chain `.closed(on_close)` to run code after every sender drops.
- For your own widget, keep a `sigil::widgets::WidgetFrame` in the builder and implement `Framed` to get the `Paint`, `States`, and `Widget` setters. See `crates/sigil/examples/story-09-custom-widget.rs`.
- Public event, command, error, and config types are `#[non_exhaustive]`. Match them with a `_` arm, and build `AppConfig` with its builder methods.

## Follow the design rules

Sigil holds the design rules in its types and in its test harness. Use the named steps. A bare pixel value does not compile where a step is expected.

- Spacing: `.gap(Space::Md)` and `.padding(Space::Lg)` take a step of the scale: `None`, `Xxs`, `Xs`, `Sm`, `Md`, `Lg`, `Xl`, `Xxl`, `Xxxl`.
- Type: `.text_role(TextRole::Title)` sets size, line height, and weight together. `.font_size(px, "reason")` needs a reason.
- Depth: `.layer(Layer::Raised)` sets fill, border, corners, and shadow together. `card` is `Raised`, and popups are `Overlay`. Do not add shadows by hand.
- Controls: a field and a button in one `row()` are the same height and are centred. Use `ControlSize` to change the height of both.
- A `Harness` checks the laid-out tree when it is dropped. A flaw fails the test, and the message names the node and the fix: clipped text, uneven controls, overlap, overflow, contrast, a small target, a broken column, or an uneven inset. Debug builds also log each flaw through `tracing` target `sigil::design`.

Fix the flaw. When a design needs the exception, say why with `.allow_flaw(Flaw::Overflow, "reason")` on the node. Call `.unaudited()` only on a test fixture that breaks a rule on purpose. With the `inspect` feature, the query `layout::audit` lists the flaws of a running app.

## Iterate on a screen without a rebuild

Put the shape of a screen in a `.sigil` file and load it with `sigil::view!("src/screen.sigil")`. In a debug build the running app shows each save within 200 ms. Bind state with `.signal`, `.flag`, and `.action`. Put a widget that the file cannot name in a `.slot`. The widgets and properties are in [Hot reload](https://raw.githubusercontent.com/manaforged/sigil/main/docs/guide/hot-reload.md).

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

The harness finds controls through the accessibility tree, runs frames until the UI settles, and runs on the terminal host by default. It needs no GPU. `Harness::expect_text` matches a node's accessible name or value; use `.get(query).value()` or `.text()` for substring assertions inside editors.

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
