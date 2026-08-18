# Owner tree

Every signal, effect, derived, interval, and node-owned subscription
belongs to a **reactive owner**. When that owner is disposed, its
children dispose first, then `on_cleanup` hooks run LIFO, then the
owner's slots go away.

This is the Leptos 0.5+ / Solid contract. It is not GPUI's entity
observer list. It is how Sigil stops leaks when a row unmounts.

## Who creates owners

- `Runtime::scope("name", \|\| { … })` creates a root and disposes it
  when the closure returns. Tests use this. `launch` uses this around
  the app.
- `Cx::add` / `Cx::add_boxed_child` create a **component owner** for
  that node. Removing the node disposes that owner.
- Each `effect` run creates a **run owner** under the effect's owner.
  The next run disposes the previous run owner first, so subscriptions
  taken inside the closure die before the next run.

You rarely construct an `Owner` by hand. The tree and the effect
runner do it.

## Cleanup

```rust,ignore
on_cleanup(|| {
    // runs when the current owner disposes
});
```

`EventChannel::subscribe` registers the same hook when a scope is
active, so disposing the view unregisters the callback even if you
drop the `Subscription` late.

`spawn` registers abort on the current owner. Dispose the owner and
the future stops. No drain required for the abort.

## Stale handles

A `Signal` is a `Copy` arena key. After dispose, `get` / `set` panic
with a stale-handle message. That is a bug in the caller: something
held a cell past its node.

`peek` and `get` do not resurrect a disposed slot.

## Cross-thread

Background work must not touch the arena. It holds an `OwnerHandle`
(identity only, `Send + Sync`). Completions post a closure through
the dispatcher. The closure runs on the main thread with that owner
current, or it is dropped if the owner is gone.

See [Async](async.md).
