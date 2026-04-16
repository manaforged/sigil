# sigil-reactive

Leptos 0.5+ style fine-grained reactive core. Signals, memoized
derived values, effects, owner tree, `on_cleanup`, `batch`, and
cycle detection. See [DR-005](../../docs/decisions/DR-005-reactivity-owner-tree.md)
for the full design.

## Status — Phase 1.1 early slice

This crate is under active development. The public API is not yet
stable. What's in place:

- `Runtime` — thread-local arena, scoped for tests and app roots
- `Owner` / `OwnerId` / `OwnerHandle` — parent-effect ownership model
- `Signal<T>` — `Copy`, arena-backed, generational identity
- `effect(f)` — subscribes to read signals, re-runs on write
- `on_cleanup(f)` — fires when the current owner disposes,
  children-first-then-self order
- `batch(f)` — coalesces signal writes so dependent effects run once
- `#[track_caller]` creation locations captured on every constructor
- Cross-thread access and wrong-thread disposal panic with owner
  name and creation location (the debuggability addendum to DR-005)

## Not yet

- `derived(f)` memoized values (Phase 1.1b)
- `dispatch_to_owner` main-thread post from background threads
  (Phase 1.1c — lands alongside `sigil-async`)
- Dev-mode leak detector and `dump_owner_tree` debug API
  (Phase 1.1d)
- `spawn_to_signal` (Phase 1.1c with `sigil-async`)

## Thread model

Everything in this crate is main-thread-only. `Signal`, `Owner`,
and the runtime arena are `!Send`. Attempting to read or write a
signal from a different thread panics in dev builds with an
actionable message naming the signal, its creation location, and
the violating thread. See DR-005 for the full contract.
