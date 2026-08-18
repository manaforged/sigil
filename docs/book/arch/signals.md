# Signals, not entities

App state is a **value graph**. It is not an object graph.

A Sigil cell holds a value. `set` is the invalidate. A button writes
it. A `derived` reads it. `text`, `label`, and `spawn` all take the
same `Copy` handle.

A GPUI `Entity` owns state, implements `Render`, and you `notify`
after you mutate it. That is the right model if the unit of UI is a
type with methods. It is the wrong model if the unit of UI is a tree
of boxes plus cells.

## The write

```rust,ignore
// Sigil — the write is the invalidate
count.set(count.peek() + 1);

// GPUI — mutate, then remember to notify
counter.update(cx, |this, cx| {
    this.n += 1;
    cx.notify();
});
```

Forgetting `notify` is a silent miss. `Signal::set` cannot forget: if
the value changed (`PartialEq`), dependents run. Equal values do not
notify. Use `set_always` when you must wake without a value change.

## Product types

One job each. Functions sit on this graph. They are not extra state
kinds.

| Type | Job |
| --- | --- |
| `Signal<T>` | Current value (level) |
| `EventChannel<T>` | Fire-once edge (menu click, tray click) |
| `Status` on a `Signal` | Async lifecycle (`spawn` writes Pending → Ok / Err) |

| Function | Job |
| --- | --- |
| `signal(v)` | Create a cell |
| `derived(\|\| …)` | Memoized read |
| `derived_eq(\|\| …)` | Same, notify only when `PartialEq` says the result changed |
| `effect(\|\| …)` | Side effect that re-runs on `.get()` deps |
| `spawn(fut)` | Background future → `Signal<Status<T, E>>` |
| `batch(\|\| …)` | Coalesce writes into one run |

`get` subscribes the current effect or derived. `peek` does not.
Handlers use `peek`. Derivations use `get`.

## What Entity still is

`sigil_reactive` still compiles `Entity`, `EntityCx`, `AsyncContext`,
and `Task`. They are `#[doc(hidden)]`. Tests and the old
`EntityCx::spawn` seam use them. Product widgets do not.

The owner tree, focus stack, action dispatch, and `DrawPhase` are
not Entity ceremony. They are shared machinery. See
[Owner tree](owners.md) and [Frame loop](frame.md).

`sigil::reactive` re-exports the signal graph only. Do not import
`AsyncContext` from that module. If a test needs the hidden types,
import `sigil_reactive` directly.

## Edges vs levels

A checkbox is a `Signal<bool>`. A menu click is not a bool you leave
sitting around. It is an edge:

```rust,ignore
let _sub = menu::events().subscribe(|id| {
    if id == "open" { /* … */ }
});
```

`EventChannel` does not retain `T` between emits. Drop the
`Subscription` (or dispose the owner) to unregister.

## Why this is the foundation

1. **The call site is a sentence.** `count.set(n)` / `text(derived(…))`.
2. **One write path.** A click and a `spawn` completion both `set`.
3. **Copy handles.** No borrow of a widget object across the tree.
4. **No second notify.** Invalidation is the write.

The rest of the architecture assumes this. Layout dirties from
reactive style and children. Emit hashes live text and reactive
slices. Accessibility peeks the same signals on paint.
