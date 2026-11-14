Sigil is a Rust UI library. One widget tree runs in a window, in a terminal, or inside another engine's frame loop.

Seven types cover most apps. Import them with `use sigil::prelude::*;` (except `Host`, which lives at `sigil::Host`).

| Type | Get one with | Use it to |
| --- | --- | --- |
| `AppConfig` | `AppConfig::new(title)` | Set the title, size, theme file, and host, then pass it to `sigil::run` |
| `NodeBuilder` | `row()`, `column()`, `container()`, or any widget builder via `.into()` | Add children, set the role and the name for screen readers, attach handlers. Every widget constructor returns a typed builder implementing `Widget`, `Paint`, `IntoNode`, and `Into<NodeBuilder>` |
| `Widget` | implemented by every builder | Set layout, style, and handlers with typed steps: `.gap(Space::Md)`, `.padding(Space::Lg)` |
| `Signal` | `signal(value)`, `memo(f)` | Hold state; a closure that calls `get` updates when the value changes |
| `View` | `sigil::view!("src/screen.sigil")` | Load a screen from a `.sigil` file and reload it on each save |
| `sigil::Host` | `sigil::Host::new(mode)` | Run a tree headless or inside another engine: mount, route events, lay out, paint |
| `Error` | any `?` | `err.kind()` says what failed |

```rust,no_run
# #[cfg(any(feature = "desktop", feature = "terminal"))]
# mod app {
use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("Counter"), counter)
}

fn counter() -> impl IntoNode {
    let count = signal(0_i32);
    column()
        .gap(Space::Md)
        .padding(Space::Lg)
        .background(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
# }
# fn main() {}
```

`AppConfig::new(title).terminal()` runs the same tree in the terminal.

## Guide

The [Sigil guide](https://github.com/manaforged/sigil/tree/main/docs/guide) covers each widget, recipes, the terminal and web hosts, hot reload, testing, and troubleshooting.

## Errors

Every fallible call returns `sigil::AppResult`. `Error::kind` returns an `ErrorKind`; branch on it, never on the message. See [Errors](https://github.com/manaforged/sigil/blob/main/docs/guide/errors.md).

## Features

Default: `desktop` (the window host). `terminal` adds the terminal host; a terminal-only program uses `default-features = false, features = ["terminal"]`. The [package README](https://github.com/manaforged/sigil/blob/main/crates/sigil/README.md#features) lists every feature.

## Stability

- The API can break between 0.x releases. Each break has a row in `CHANGELOG.md`.
- Public event, command, error, and config enums are `#[non_exhaustive]`. Match them with a `_` arm.
- Items outside the prelude, the crate root, and the public modules' documented items are not API. Items public only under the `kittest` feature are test support and not API.
- MSRV is Rust 1.96.
