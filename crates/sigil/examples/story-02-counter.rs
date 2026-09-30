use sigil::prelude::*;

fn main() -> AppResult {
    run("Counter", counter)
}

pub(crate) fn counter() -> impl IntoNode {
    let count = signal(0);
    row()
        .child(button("-").activate(move || count.update(|n| *n -= 1)))
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("+").activate(move || count.update(|n| *n += 1)))
}
