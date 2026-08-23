# Sigil — Quickstart

> Book status: written for `sigil-ui 0.1.0-alpha.1`. Every example on this
> page compiles in `examples/` (built by the workspace check gate).

Sigil is a Rust-native desktop UI framework: signal-graph reactivity, an
owned incremental layout engine, a wgpu renderer with an owned text
pipeline, AccessKit accessibility and IME, a utility-class style
layer, and a widget catalog in the crate (buttons, fields, tables,
lists, charts, menus). You write plain Rust against one import. Widget helpers are
`IntoNode`; type a layout helper as `impl IntoNode`. `launch` and
`run_tui` take a closure, `text`, `container`, or `root(button("Go"))`.

```rust,ignore
use sigil::prelude::*;
```

## Install

```toml
[dependencies]
sigil-ui = "0.1.0-alpha.1"
```

The package is `sigil-ui` (the bare `sigil` name was taken on crates.io);
the library is still `sigil`, so imports read `sigil::…` as usual.

## The smallest app

A complete reactive counter in ~30 lines (`examples/hello-counter`):

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

The four moving parts:

- **`sigil::launch((title, width, height), app_tree)`** opens a window
  and drives the frame loop. `app_tree` builds the node tree once.
- **`signal(0)`** creates a reactive cell. Signals are `Copy` — pass
  them into closures freely.
- **`derived(...)`** computes a value from signals and re-evaluates when
  they change. `text(label_text)` re-renders for free when the count
  changes — no subscriptions, no repaints to schedule.
- **`.activate(move || ...)`** is an event handler: a plain closure
  that writes signals. `count.peek()` reads without subscribing the
  enclosing effect; `count.get()` reads with subscription.

## Three hosts

The tree is the same. The entry point changes.

| You want | You call |
|---|---|
| A desktop window | `sigil::launch` (`examples/hello-counter`) |
| Your own loop | `Host` inside `Runtime::scope` (`examples/hello-host`) |
| A character grid | `sigil::run_tui` (`examples/hello-tui`, feature `tui`) |

`signal` panics outside `Runtime::scope`. `launch` and `run_tui` open
that scope. An embed host that uses signals must call it.

`Host::scene` needs feature `desktop` or `render`. Without it, use
`Host::painted`. After layout, `Host::surfaces` lists `surface(key)`
holes. You place a native view. See [The application host](07-application.md).

## Styling in one breath

`classes!("p-8 bg-surface rounded-md")` is the default styling path:
Tailwind-shaped utility tokens, **validated at compile time** (a typo
fails the build with a nearest-match suggestion). Spacing tokens are
4px steps (`p-4` = 16px, `w-4` = 16px); `opacity-50` is a percent. Theme
tokens (`bg-surface`, `text-on-surface`, `bg-accent`) come from the
active theme — dark mode is a theme swap, not a restyle. Chapter 2
covers the full system.

## Interaction in one breath

Static styling works on any node; interaction styling goes through
`interactive()` — the opt-in wrapper that gives plain containers hover,
press, focus, and disabled semantics (`examples/hello-style`):

```rust,ignore
interactive(
    container()
        .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
        .class(classes!("p-8 bg-accent rounded-md")),
)
.hover(|s| s.bg("accent-hover"))
```

No pointer math, no manual hit-testing, no signal plumbing for a hover
background. Widgets carry their own interaction model — `button` ships
working `.hover()` / `.focus()` / `.pressed()` / `.disabled_style()`
closures out of the box.

## Where to go next

- **Reactive model**: effects, batching, `spawn`.
- **The box**: one `LayoutStyle`.
- **Styling**: cascade, themes, tokens.
- **Widgets**: `button`, `icon_button`, `text_field`, `text_edit`,
  `modal` / `sheet`, `virtual_list`, `table`, `card`, `avatar`,
  `sidebar`. Native alert is `sigil::dialog::message` / `confirm`.
- **Testing** and **Profiling**.
- **Architecture**: why signals, the frame loop, crate map.

The catalog in one window: `examples/widget-playground`.

Build the book with `mdbook serve docs/book`.
