# The reactive model

Sigil's reactivity is a main-thread signal graph: signals hold values,
effects run when their dependencies change, and the frame loop paints
whatever changed. No virtual DOM, no diffing, no per-frame rebuild.

## Signals

```rust,ignore
let count = signal(0_i32);
count.set(count.peek() + 1);   // write
let now = count.get();         // read (subscribing)
let later = count.peek();      // read (untracked)
```

- Signals are **`Copy`** — hand them into closures by move, clone-free.
- **`.get()`** subscribes the enclosing effect/derived; **`.peek()`**
  reads without subscribing. The rule of thumb: `get` in derivations,
  `peek` in event handlers.
- **`.update(|v| *v += 1)`** for in-place mutation.
- Measured costs (from `docs/benchmarks/baseline-2026-04-14.md`):
  `signal_get` 5.4 ns, `set` with one subscriber ~120 ns, zero-alloc
  sustained streaming.

## Derived values

```rust,ignore
let label = derived(move || format!("Count: {}", count.get()));
text(label)  // re-renders whenever count changes — no wiring
```

`derived` memoizes a computation over signals. Text nodes accept
derived signals directly; re-render is automatic.

## Effects

```rust,ignore
effect(move || {
    if enabled.get() {
        log_state();
    }
});
```

Effects run once at registration and again whenever a dependency they
read with `.get()` changes. Effects own their subscriptions: each re-run
disposes and recreates its owner, so cleanups stay local
(`on_cleanup(...)` for teardown).

## Batching

Independent writes in one handler coalesce into one effect run and one
repaint:

```rust,ignore
batch(|| {
    first.set(1);
    second.set(2);
});  // dependents run once, after the batch
```

## Events are plain closures

```rust,ignore
button("Save").activate(move || save_signal.set(true));
container().pointer_down(move |event| { /* ... */ });
```

Handlers write signals; the frame that delivered the event lays out and
paints the result in the same pass. Pointer enter/leave, down/up, move,
scroll, key (incl. `Shortcut`), focus in/out, IME, and lifecycle events
are all on `NodeEventsExt` (in the prelude).

## Cross-thread work

```rust,ignore
let handle = cx.spawn(async move { fetch_rows().await });
// completion lands on the main thread's reactive queue next frame
```

`sigil-tasks` owns the Tokio runtime; completions dispatch back to the
owning main thread (owner-targeted cancellation — dropping the owner
cancels the task). The wake path is wired into the winit loop, so
streamed updates paint promptly.

## Testing reactivity

Everything above runs headless:

```rust,ignore
Runtime::scope("my-test", || {
    let mut cx = Cx::new_root();
    let root = cx.add(/* tree */);
    // drive signals, route synthetic events, assert on the tree
});
```

Chapter 4 covers the full testing surface (layout compute, event
routing, goldens).
