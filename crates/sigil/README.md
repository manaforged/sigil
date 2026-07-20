# sigil-ui

`sigil-ui` is the facade crate for **sigil**, a Rust-native desktop UI
framework: signal-graph reactivity, an owned incremental layout engine, a
wgpu renderer with an owned text pipeline, AccessKit accessibility and IME,
and a utility-class style layer with design tokens.

> The package is named `sigil-ui` because the bare `sigil` name was already
> taken on crates.io. The library is still `sigil`, so every import reads
> `sigil::…` as usual.

## Example

```rust,no_run
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
            .class("p-8 bg-surface")
            .child(text(label_text).class("text-on-surface"))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

## Macro note

`classes!` and `#[derive(Action)]` expand to paths under
`sigil::__private::…`. They therefore require depending on this facade
crate — not on the `sigil-*` leaf crates directly — and the dependency
must keep the name `sigil-ui` (the library name `sigil` is what expansions
resolve).

## Feature flags

- `render` — expose `sigil::{render, text, ui}`: the Slug text pipeline and
  scene-compositing surface for embedding a host engine's frame loop
  (DR-054).

License: MIT OR Apache-2.0.
