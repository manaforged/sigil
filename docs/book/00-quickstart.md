# Sigil — Quickstart

> Book status: written for `sigil-ui 0.1.0-alpha.1`. Every example on this
> page compiles in `examples/` (checked in CI).

Sigil is a Rust-native desktop UI framework: signal-graph reactivity, an
owned incremental layout engine, a wgpu renderer with an owned text
pipeline, AccessKit accessibility and IME, and a utility-class style
layer. You write plain Rust against one import:

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
use sigil::reactive::derived;

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

- **Reactive model** (chapter 1): effects, batching, ownership,
  cross-thread tasks.
- **Styling** (chapter 2): the cascade, themes, tokens, state styling,
  the fluent APIs.
- **Widgets** (chapter 3): the catalog — table, virtual_list, text
  editing, overlays.
- **Testing** (chapter 4): headless `sigil::testing`, goldens, writing
  your own harnesses.
