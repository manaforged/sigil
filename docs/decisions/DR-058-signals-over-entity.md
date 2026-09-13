---
id: DR-058
status: accepted
date: 2026-08-19
covers: Signals as the product surface — Entity/notify demoted to shared machinery
supersedes: DR-045 (unavailable — local-only era, see README)
relates: [DR-040, DR-046, docs/book/arch/state.md]
---

# DR-058 — Signals over entities

**Status:** Accepted (supersedes DR-045)
**Date:** 2026-08-19

## Context

DR-045 (2026-05-18) adopted GPUI's `Entity<T>` / `cx.notify()` shape verbatim
as sigil's reactivity surface, reasoning that mirroring GPUI's validated model
would ease adoption. In practice the signal graph (`signal` / `derived` /
`effect` / `batch`) became the surface every widget, example, dogfood app, and
book chapter actually uses, while `Entity` survived only as plumbing under
`spawn` and `EventChannel`. The Aug-18 widget-extraction commit (50ce481)
made the split explicit: the book chapter is "Signals, not entities", and
`Entity`, `EntityCx`, `AsyncContext`, and `Task` are `#[doc(hidden)]` in
`crates/sigil/src/reactive.rs`.

The DR trail never recorded the pivot: DR-045 still read "accepted" while the
public surface contradicted it.

## Decision

- The signal graph is the product reactivity surface. New widgets, examples,
  and docs use `signal` / `derived` / `effect`; the prelude exposes only that
  surface.
- `Entity<T>` / `EntityCx` / `AsyncContext` / `Task` remain as internal
  shared machinery (spawn seams, event channels) behind `#[doc(hidden)]`.
  They are not part of the authored API and carry no stability promise in
  0.1.x.
- DR-045 is superseded for the authored-API question. Its observer-primitive
  machinery (watchers, emission ordering) survives as implementation detail.

## Consequences

- Forgetting `notify` is no longer a failure mode user code can express —
  the silent-miss class GPUI leaves to convention is closed by construction.
- Any future re-exposure of `Entity` as public API needs a new DR, not a
  revival of DR-045.
