# State

App state is a signal graph. A signal holds a value. A write to the signal is the invalidation; there is no separate notify call.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let count = signal(0_i32);
let label = memo(move || format!("Count: {}", count.get()));
button("Increment").activate(move || count.set(count.peek() + 1));
# };
```

`get` subscribes the current effect or derived. `peek` reads without subscribing. Event handlers use `peek`; derived values use `get`. A `set` with an equal value does not notify (`PartialEq`). `update` always notifies, so `update(|v| *v = next)` wakes subscribers without a value change and writes a type that is not `PartialEq`.

| Function | Job |
| --- | --- |
| `signal(v)` | Create a cell |
| `memo(\|\| ...)` | Memoized read; notify only when the result changes |
| `effect(\|\| ...)` | Side effect that re-runs on its `get` dependencies |
| `task(make)` | Async work on demand → `Task`; `run()`, `autostart()`, `cancel()`, `status()` |
| `spawn(fut)` | Background future → `Signal<Status<T, E>>` |
| `batch(\|\| ...)` | Coalesce writes into one run |

`EventChannel<T>` is a fire-once edge, for a menu click or a tray click. It does not retain `T` between emits. Drop the subscription, or dispose the owner, to unregister.

## Owners and cleanup

Every signal, effect, memo, interval, and subscription belongs to a reactive owner. When an owner is disposed, its children dispose first. Then the `cleanup` hooks run in last-in-first-out order. Then the owner's slots go away.

- `Owner::scope("name", || { ... })` creates a root. `run` wraps the app in one. Tests use it.
- `Cx::add` creates one owner per node. Removing the node disposes the owner and its subtree.
- Each effect run creates an owner for that run. The next run disposes the previous one, so subscriptions taken inside the closure stop first.
- An event handler, such as `.activate`, `.change`, or `.key`, runs with the owner of its node as the current owner. `signal`, `spawn`, and `interval` work inside a handler. What they create belongs to that node and lives until the node is removed, so a handler that runs many times creates a new item on each run. Create long-lived state in the component and write it from the handler.

`signal`, `memo`, `effect`, `spawn`, and `interval` panic when no owner is current.

`cleanup(|| { ... })` registers cleanup for the current owner. `EventChannel::subscribe` registers the same hook in an active scope. `spawn` registers an abort on the current owner. `Task::run` registers one on the owner that created the task.

A `Signal` is a `Copy` arena key. After dispose, `get` and `set` panic with a stale-handle message. `peek` and `get` do not revive a disposed slot.

## Async

`spawn` returns a `Signal<Status<T, E>>`. The signal starts `Status::Pending`. The future's output is `Result<T, E>`. On completion, the main-thread dispatcher writes `Ok` or `Err`. If the future panics, the dispatcher writes `Status::Panicked` with the panic message. `panic_message(payload)` returns that text for any panic payload. A cancelled task does not write `Err`; the signal stays `Pending` and the task drops.

`task(make)` returns a `Copy` `Task` whose status stays `Status::Idle` until the first run. `make` is a `Fn() -> Fut + 'static` closure, and `Fut` is a `Send + 'static` future whose output is `Result<T, E>`, with `T` and `E` both `Send + 'static`. `E` needs no error trait, so a `String` works. `task(make).autostart()` starts that run when the task is created. Each `run()` writes `Pending`, aborts the previous run, and spawns `make()`. `run()` returns `()`; read the outcome from `status()`. A result writes only if no later run has started, so a replaced run's result drops even when it was already queued. `cancel()` aborts the run and sets a `Pending` status back to `Idle`.

`status()` returns a clone of the current `Status<T, E>`, so it needs `T: Clone` and `E: Clone`. It reads with `get`: a call inside an effect, a `memo`, or a `text(move || ...)` closure subscribes that closure, which runs again when the status changes.

Background work must not touch the arena. It holds an `Owner`, which carries identity only and is `Send + Sync`. Completions post a closure through the dispatcher. The closure runs on the main thread with that owner current, or it is dropped if the owner is gone.

Use `task` for work that runs on demand and may run again, and `spawn` for a future that starts now and owns state it cannot rebuild. A worker that already exists writes through `signal.writer()`, which keeps the current owner and lets the worker write repeatedly, or sends on a channel that `listen` drains on the main thread. `listen(rx, f).closed(done)` runs `done` once after every sender drops.
