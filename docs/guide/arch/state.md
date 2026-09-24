# State

App state is a signal graph. A signal holds a value. A write to the
signal is the invalidation; there is no separate notify call.

```rust,ignore
let count = signal(0_i32);
let label = derived(move || format!("Count: {}", count.get()));
button("Increment").activate(move || count.set(count.peek() + 1));
```

`get` subscribes the current effect or derived. `peek` reads without
subscribing. Event handlers use `peek`; derived values use `get`. A
`set` with an equal value does not notify (`PartialEq`). Use
`set_always` to wake subscribers without a value change.

| Function | Job |
| --- | --- |
| `signal(v)` | Create a cell |
| `derived(\|\| ...)` | Memoized read |
| `derived_eq(\|\| ...)` | Memoized read; notify only when the result changes |
| `effect(\|\| ...)` | Side effect that re-runs on its `get` dependencies |
| `spawn(fut)` | Background future → `Signal<Status<T, E>>` |
| `batch(\|\| ...)` | Coalesce writes into one run |

`EventChannel<T>` is a fire-once edge, for a menu click or a tray
click. It does not retain `T` between emits. Drop the subscription, or
dispose the owner, to unregister.

## Owners and cleanup

Every signal, effect, derived, interval, and subscription belongs to a
reactive owner. When an owner is disposed, its children dispose first.
Then the `on_cleanup` hooks run in last-in-first-out order. Then the
owner's slots go away.

- `Runtime::scope("name", || { ... })` creates a root. `run` wraps
  the app in one. Tests use it.
- `Cx::add` creates one owner per node. Removing the node disposes the
  owner and its subtree.
- Each effect run creates an owner for that run. The next run disposes
  the previous one, so subscriptions taken inside the closure stop
  first.

`on_cleanup(|| { ... })` registers cleanup for the current owner.
`EventChannel::subscribe` registers the same hook in an active scope.
`spawn` registers an abort on the current owner.

A `Signal` is a `Copy` arena key. After dispose, `get` and `set` panic
with a stale-handle message. `peek` and `get` do not revive a disposed
slot.

## Async

`spawn` returns a `Signal<Status<T, E>>`. The signal starts
`Status::Pending`. The future's output is `Result<T, E>`. On
completion, the main-thread dispatcher writes `Ok` or `Err`. If the
future panics, the dispatcher writes `Status::Panicked` with the panic
message. `panic_message(payload)` returns that text for any panic
payload. A cancelled task does not write `Err`; the signal stays
`Pending` and the task drops.

Background work must not touch the arena. It holds an `OwnerHandle`,
which carries identity only and is `Send + Sync`. Completions post a
closure through the dispatcher. The closure runs on the main thread
with that owner current, or it is dropped if the owner is gone.

`dispatch_to_owner` posts a `FnOnce()` to the main thread. Use it when
a worker already exists and only needs to write signals. Use `spawn`
when you have a future. `signal_writer(signal)` keeps the current owner
and lets a worker write repeatedly.
