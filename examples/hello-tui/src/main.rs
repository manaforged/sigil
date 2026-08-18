//! The hello-counter tree on `sigil::run_tui`.
//!
//! Same signal, derived label, and button. The host is cells, not wgpu.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run_tui(("Sigil - hello-tui", 80, 24), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label_text = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().gap(1.0))
            .child(text(label_text))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
