# DR-003 — Layout: Taffy upstream behind HAL, no fork day one

**Status:** Accepted
**Date:** 2026-04-14

## Context

Taffy is the Rust layout engine used by Blitz, Bevy, Slint, Floem, Zed
(reportedly), Dioxus, and others. It implements CSS Block, Flexbox, and
Grid layout.

An earlier draft of this plan proposed forking Taffy on day one, citing:

- Zed's (unverified) fork
- Missing CSS features: `inline-block`, `content-box`, full float support
- A desired fast-path for "flex row of N items with one main-axis grow"
  specialized for our common case

Research findings (April 2026):

- The "Zed maintains a Taffy fork" claim is **unverified**. We could not
  find the public repo or a 2025–2026 reference.
- Taffy upstream is still fixing base-algorithm correctness bugs:
  flex-grow with padded parents (#909), block `width: auto` not filling
  available space (#908), grid percentage resolution (#920), nested
  absolute positioning (#923).
- A fork would inherit all of those bugs and have to track upstream
  forever. The specialized flex fast-path needs **measured evidence**
  on real reference-app layouts before we commit to owning code.
- Blitz drives upstream priorities toward browser-style spec compliance.
  Good for correctness; uninterested in our component-library fast path.

## Decision

Depend on `taffy` upstream. Wrap behind a `Layout` trait in `sigil-hal`.

**No fork on day one.** The HAL trait is the swap point: if a measured
bottleneck later justifies a specialized implementation, we build it
against the same trait without touching consumers.

## Consequences

- We inherit Taffy's open correctness bugs. Our adapter includes a fixture
  test suite (ported from WPT where possible) so we notice regressions.
- `sigil-layout` is a thin adapter, not an engine. It normalizes Taffy's API
  quirks and exposes the `Layout` trait implementation.
- If the fast-path need materializes, it becomes a separate crate
  (`sigil-layout-fast`) behind the same trait, not a fork.

## Kill criteria

Fork (or replace) Taffy when, and only when, all of:

1. A measured layout bottleneck in a real consumer (likely the virtualized
   data grid in Phase 7) traces to Taffy's algorithm, not to our adapter
   or draw calls.
2. The measurement shows the bottleneck is not fixable by cache strategy,
   partial layout, or subtree invalidation.
3. An upstream fix is not on Taffy's roadmap and an upstream maintainer
   confirms it won't be accepted.

Fork with a new DR and full benchmark evidence.

## References

- [Taffy](https://github.com/DioxusLabs/taffy)
- [Taffy issue #908 — block width auto](https://github.com/DioxusLabs/taffy/issues/908)
- [Taffy issue #917 — small-scope recalculation perf](https://github.com/DioxusLabs/taffy/issues/917)
