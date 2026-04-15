# DR-005 — Reactivity: Leptos 0.5+ owner tree

**Status:** Accepted
**Date:** 2026-04-14

## Context

Every serious frontend framework between 2022 and 2026 converged on
fine-grained signals: Solid, Vue 3, Angular, Svelte 5, Preact, Leptos,
Floem, Dioxus. TC39 Signals reached Stage 1. We adopt the consensus.

The interesting design question is not "signals yes or no" — it is
**ownership and disposal**. Research findings:

- **Leptos 0.5** (2023) replaced its pre-0.5 `Scope`-based model with a
  "parent effect/memo owns" owner tree. Quote from the life-cycle
  appendix: "rather than being owned by a Scope, every signal, effect,
  or memo is now owned by its parent effect or memo... Arena-allocated
  signals are Copy and disposed when their reactive Owner cleans up."
  The motivation was documented memory leaks in the pre-0.5 model.
- `on_cleanup` contract: "runs when the current reactive owner is
  disposed, running after child nodes have been disposed, but before
  signals, effects, and resources are invalidated." This ordering is
  load-bearing for async cancellation and resource release.
- Solid.js is the direct ancestor: owner tree, `onCleanup`, `createRoot`,
  `untrack`, `batch`. This model is the industry consensus.
- **TC39 Signals explicitly refuses to standardize ownership and
  disposal**, deferring to frameworks: "does not attempt to address
  framework-specific areas such as effects, scheduling, and
  ownership/disposal." There is no platform answer coming.
- No existing signal library ships **dev-time leak detection** as a
  first-class feature. This is a genuine gap.
- Coarse events (window resize, theme switch, locale change) fan out
  through fine-grained reactivity and can cause performance regressions
  — Floem #1016 (WindowResized triggers full view-tree traversal) is a
  direct warning.

## Decision

Adopt the Leptos 0.5+ owner tree model:

- Three primitives: `signal`, `derived` (memoized), `effect`.
- `Signal<T>: Copy + 'static` — a handle (index + generation) into a
  thread-local arena owned by the active app context.
- Every signal, derived, and effect is owned by its parent reactive
  owner. When an owner is disposed, its children are disposed first,
  then its own resources.
- `on_cleanup(f)` registers teardown, runs after child owners are
  disposed, before the current owner's signals are invalidated.
- Keyed lists (`For`, `Index`) create per-item owners. Item removal
  disposes the item's owner, cleaning up its effects, resources, and
  spawned tasks.
- Async cancellation is tied to owner disposal (see DR-007).
- Batched writes via `batch(|| ...)`.
- Untracked reads via `signal.peek()`.
- Cycle detection panics with a clear message in dev, warns in release.

**Differentiators:**

- **Dev-time leak detector.** The inspector walks the owner tree,
  reports orphaned signals and unfired cleanups, and warns on
  suspiciously long-lived effects. No existing signal library ships
  this.
- **Scoped invalidation channels** for coarse events (resize, theme,
  locale, DPI, high-contrast). Dedicated broadcast signals that only
  affected subscribers observe. Mitigates the Floem #1016 class of bug.

## Consequences

- Components run once at build and never re-run. Updates flow through
  signal subscriptions, touching only the specific bindings that read
  the changed value. No VDOM, no view diffing.
- Leptos 0.5's documented ownership awkwardness for library authors
  (Leptos issue #3451) applies to us too. We will likely ship helper
  utilities (`Root`, `Scoped<T>` equivalents) as the API matures.
- The thread-local arena means reactive operations from non-main threads
  panic. Background work uses the dispatcher (DR-007).

## Addendum — Debuggability of thread-local ownership

**Added 2026-04-14.** The decision above commits to a thread-local
current-owner stack (Leptos/Solid style) for ergonomics: `signal(0)`
and `effect(|| ...)` work without threading a `Cx` through every call
site. The trade-off usually cited is that cross-thread debuggability
suffers — stale handles, leaked owners, and wrong-thread accesses all
surface as "arena slot invalid" instead of something actionable.

We refuse that trade-off. The model is **thread-local for ergonomics,
explicit-at-the-seams for control, and richly introspectable
everywhere**. Concretely:

### 1. Every owner has identity

`OwnerId` (stable, non-reusing) + a `DebugName: &'static str` in
release, `String` in dev. The name is set at creation time:
`Owner::new_with_name("Button(id=42)")`, `"App::root"`,
`"For<Account>[3]"`. `Signal`, `Effect`, and `Derived` carry their
owner id and their own debug name (`"Button.hover_style"`).

### 2. Every owner records its creation location in dev

`#[track_caller]` on `signal`, `effect`, `derived` captures
`core::panic::Location::caller()` at construction time. Leak detector
output, cleanup-ordering panics, and the inspector all report the
file:line where the offending owner was born.

### 3. `with_owner` (main-thread) and `dispatch_to_owner` (cross-thread)

There are two distinct surfaces here and conflating them is the
mistake to avoid:

**`Owner` is `!Send`, main-thread-only.** Holding one lets the caller
observe and mutate reactive state. It cannot cross a thread boundary.
Background code cannot carry an `Owner` and cannot run reactive ops.

**`OwnerHandle` is `Send + Sync + Clone`.** It is an identity-only
weak reference — an `OwnerId` plus a generation counter. It grants no
reactive access on its own. Background code carries handles, never
owners.

On the main thread, `with_owner(&owner, || { ... })` re-enters a
specific owner for the duration of a synchronous closure. Library
authors and tests use this for explicit scoping without threading
`Cx` through every call.

Across threads, background code calls `dispatch_to_owner(&handle,
|scope| { ... })` which posts the closure through the main-thread
dispatcher (DR-007). The closure runs later, on the main thread, with
the named owner active. If the owner has already been disposed between
enqueue and drain, the dispatcher drops the closure — no panic, no
leaked cleanup registration. Cancellation is an identity check at
drain time, not a cross-thread reactive operation:

```rust
let handle = Owner::current().handle();
app.spawn(async move {
    let data = fetch().await;
    dispatch_to_owner(&handle, move |scope| {
        // runs on the main thread, with `scope` as the current owner.
        // if the owner has been disposed, this closure is dropped.
        scope.write(result_signal, data);
        on_cleanup(|| { /* registered against `scope` */ });
    });
});
```

This keeps the main-thread-only invariant intact — reactive operations
never happen off-thread — while still giving background code a
deterministic way to commit results into a specific owner's scope.
The disposal/cancellation invariant is: **an `OwnerHandle` whose
owner has been disposed is a no-op target, not a panic.**

### 4. Wrong-thread access panics with actionable messages

Every `Signal` handle carries its creation `ThreadId`. Dev-build reads
and all writes check it. The panic is not "arena slot invalid" — it is:

```
reactive access from wrong thread:
  signal `counter` (created at src/counter.rs:42 in owner `App::root`)
  read from thread `tokio-worker-3`

reactive reads and writes must happen on the main thread.
use `spawn_to_signal(...)` for background work that updates signals,
or `with_owner(&owner, || ...)` to scope cleanups to a specific owner.
```

### 5. Dev-mode owner-tree dump

`sigil::debug::dump_owner_tree()` returns a human-readable tree
keyed by `DebugName`, with per-owner signal and effect counts:

```
App::root
├─ Window(main)
│  ├─ Button(submit)        [signals: hovered]
│  ├─ For<Account>          [owners: 10]
│  │  ├─ Row(0)             [signals: selected; effects: 1]
│  │  ├─ Row(1)             ...
```

Available as a method on `App`, as a command in the inspector
(Phase 11), and as a `Debug` impl on `Owner` for test assertions.

### 6. Effect trace log behind an env flag

`RUST_UI_TRACE_EFFECTS=1` prints every effect firing with its debug
name, the signals read, and the signals written. Answers "why did
this re-render" questions without a debugger.

### 7. Leak detector

In dev, dropping an owner whose cleanups never fired (typically
because its parent was force-dropped without disposal) logs a warning
naming every orphaned owner by `DebugName` and creation location.
No existing signal library ships this; TC39 Signals explicitly refuses
to standardize it; we fill the gap.

### Cost and scope

- ~24 bytes per owner in release (`ThreadId`, `OwnerId`, name ptr+len).
  ~200 bytes in dev if a `Backtrace` is attached.
- One thread-local read per reactive construction. Free.
- All debug checks behind `cfg(debug_assertions)` where cost is
  non-trivial. Release builds pay only for the identity fields.

This addendum is not a new decision — it is a refinement of how the
owner tree is implemented. The owner model itself remains
Leptos 0.5+ style. We implement this in the first `ui-reactive`
commit, not as a later polish pass, because naming and location
capture are load-bearing for every debug tool that follows and
retrofitting them later means touching every reactive construction
site.

## Kill criteria

Reconsider if:

- TC39 Signals advances past Stage 2 with a standardized ownership
  model that materially differs from Leptos 0.5's.
- A correctness or performance issue in the owner tree model is not
  fixable without rewriting the core (Leptos's own rewrite was such a
  moment — we learn from it, not repeat it).

## References

- [Leptos — Appendix: Life Cycle of a Signal](https://book.leptos.dev/appendix_life_cycle.html)
- [Leptos issue #3451 — Scoped<T> utility](https://github.com/leptos-rs/leptos/issues/3451)
- [Leptos discussion #2552 — Non-'static signals](https://github.com/leptos-rs/leptos/discussions/2552)
- [TC39 Signals proposal](https://github.com/tc39/proposal-signals)
- [Floem issue #1016 — WindowResized view traversal](https://github.com/lapce/floem/issues/1016)
