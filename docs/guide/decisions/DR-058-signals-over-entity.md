---
id: DR-058
status: accepted
covers: Signals as the product surface; Entity and notify are shared machinery
relates: [DR-040, DR-046, docs/guide/arch/state.md]
---

# DR-058: Signals over entities

**Status:** Accepted

## Context

Every widget, example, and guide chapter uses the signal graph (`signal`, `derived`, `effect`, `batch`). `Entity` is only plumbing under `spawn` and `EventChannel`. `Entity`, `EntityCx`, `AsyncContext`, and `Task` are `#[doc(hidden)]` in `crates/sigil/src/reactive.rs`.

## Decision

- The signal graph is the product reactivity surface. New widgets, examples, and docs use `signal`, `derived`, and `effect`. The prelude exposes only that surface.
- `Entity<T>`, `EntityCx`, `AsyncContext`, and `Task` remain as internal shared machinery (spawn seams, event channels) behind `#[doc(hidden)]`. They are not part of the authored API and carry no stability promise in 0.1.x.
- The observer machinery (watchers, emission order) stays as an implementation detail.

## Consequences

- User code cannot forget to call `notify`. The type system prevents it.
- To expose `Entity` as public API again, write a new DR.
