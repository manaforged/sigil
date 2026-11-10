A reactive value. A closure that calls `get` runs again when the value changes.

`signal(value)` creates one. `get` reads and subscribes, `peek` reads without subscribing, and `set` and `update` write. `set` skips a value equal to the current one. `derived(f)` is a signal that `f` recomputes, and `effect(f)` runs `f` after each change it reads. Write a signal on the UI thread; from another thread use `signal_writer`.

A write outside an effect runs each closure that depends on it one time before the write returns. A closure that reads two derived values runs after both have their new value.

```rust
use sigil::prelude::*;

fn counter() -> impl IntoNode {
    let count = signal(0_i32);
    let even = derived(move || count.get() % 2 == 0);
    row()
        .child(text(move || format!("{} even={}", count.get(), even.get())))
        .child(button("+").activate(move || count.update(|n| *n += 1)))
}
```

`signal`, `derived`, and `effect` panic outside an owner scope. `sigil::run` opens one; an embed host calls `sigil::reactive::Owner::scope`.

See also: [`Host`](crate::Host), [State](https://github.com/manaforged/sigil/blob/main/docs/guide/arch/state.md).
