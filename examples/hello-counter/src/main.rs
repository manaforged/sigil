//! Minimum viable reactive counter on `sigil::launch`.
//!
//! A `Signal<i32>` holds the count. A button increments it. A reactive
//! label derived from the count re-renders on every change. This is the
//! "hello world" of Sigil's reactive UI: one signal, one event handler,
//! one derived view.

#![allow(missing_docs)]

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
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(480.0, 240.0)
                    .gap(16.0),
            )
            .class("p-8 bg-surface")
            .child(text(label_text).class("text-on-surface"))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
