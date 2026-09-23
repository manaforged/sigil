# Quickstart

> Sigil is unreleased. The examples on this page are included in the
> framework repository.

Sigil is a desktop UI library in Rust. State is a signal graph. Layout,
text shaping, and the wgpu renderer are owned code. AccessKit, IME,
utility classes, and the widget catalog ship in the crate. You write
plain Rust against one import.

```rust,ignore
use sigil::prelude::*;
```

## Install

Sigil is not published on crates.io. The repository is currently private;
you need GitHub access and Git authentication to use this dependency.

```toml
[dependencies]
sigil-gui = { git = "https://github.com/manaforged/sigil", rev = "<sha>" }
```

The package is `sigil-gui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`.
`Cargo.lock` records the selected Git commit. For a terminal-only
program, use the dependencies in [Terminal host](terminal.md).

## The smallest app

A complete reactive counter (example `hello-counter`):

```rust,ignore
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label_text = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 240.0).gap(16.0))
            .class(classes!("p-8 bg-surface"))
            .child(text(label_text).class(classes!("text-on-surface")))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

The moving parts:

- `sigil::launch((title, width, height), app_tree)` opens a window and
  drives the frame loop. `app_tree` builds the node tree once.
- `signal(0)` creates a reactive cell. Signals are `Copy`; pass them
  into closures freely.
- `derived(...)` computes a value from signals and re-evaluates when
  they change. `text(label_text)` re-renders when the count changes.
  You do not subscribe or schedule a repaint.
- `.activate(move || ...)` is an event handler: a plain closure that
  writes signals. `count.peek()` reads without subscribing the
  enclosing effect; `count.get()` reads with subscription.

Widget helpers implement `IntoNode`; type a layout helper as
`impl IntoNode`. `launch` takes a closure, `text`, `container`, or
`root(button("Go"))`.

## Three hosts

The tree is the same. The entry point changes.

| You want | You call |
|---|---|
| A desktop window | `sigil::launch` (`hello-counter`) |
| Your own loop | `Host` inside `Runtime::scope` (`hello-host`) |
| A character grid | `sigil_tui::run` (crate `sigil-tui`, `hello-tui`) |

`signal` panics outside `Runtime::scope`. `launch` and `sigil_tui::run`
open that scope. An embed host that uses signals must call it.

`Host::scene` needs feature `desktop` or `render`. Without it, use
`Host::painted`. After layout, `Host::surfaces` lists `surface(key)`
holes. You place a native view. The `sigil-gui` README covers the embed
details.

## Styling

`classes!("p-8 bg-surface rounded-md")` is the default styling path.
The tokens are Tailwind-shaped and parsed at runtime; a typo warns and
is ignored. Spacing tokens are 4px steps (`p-4` = 16px, `w-4` = 16px).
`opacity-50` is a percent. Theme tokens (`bg-surface`, `text-on-surface`,
`bg-accent`) come from the active theme. Dark mode is a theme swap, not
a restyle.

A theme can also live in a TOML file: `base = "dark"` (or `"light"`,
`"console_dark"`) plus a `[tokens]` table of hex colors
(`accent = "#7c3aed"`). `AppConfig::theme_file(path)` loads it at launch
and keeps watching it — edits re-apply on the next frame, and writes in
quick succession coalesce into one reload. For manual control,
`sigil::watch_theme(path)` returns a `ThemeWatcher` guard that re-parses
the file and calls `set_theme` on the UI thread until dropped; call it
inside the app tree so the reload can reach the UI thread.

## Interaction

Static styling works on any node. Interaction styling goes through
`interactive()`, the opt-in wrapper that gives plain containers hover,
press, focus, and disabled semantics:

```rust,ignore
interactive(
    container()
        .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
        .class(classes!("p-8 bg-accent rounded-md")),
)
.hover(|s| s.bg("accent-hover"))
```

`interactive()` owns the hit testing and the state, so a hover
background needs no signal of your own. `button` already defines
`.hover()`, `.focus()`, `.pressed()`, and `.disabled_style()` closures
out of the box.

## Where to go next

- Widgets: `button`, `icon_button`, `text_field`, `text_edit`,
  `modal` / `sheet`, `virtual_list`, `table`, `card`, `avatar`,
  `sidebar`. Native alert is `sigil::dialog::message` / `confirm`.
- Architecture: [Overview](arch/overview.md),
  [State](arch/state.md), [Painting](arch/painting.md),
  [Platform](arch/platform.md), [Modules](arch/modules.md).
- The runnable widget demos: `crates/sigil/examples`.
- The terminal host: the `sigil-tui` crate.

Build the book with `mdbook serve docs`.
