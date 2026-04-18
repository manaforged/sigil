# DR-024 — Layout cache-key content hash (supersedes DR-023 cache shape)

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes in part:** DR-023 (retains Variant B as the *strategy*; replaces the specific cache-key shape and halt conditions).
**Depends on:** DR-015 (`Cx::mark_mutated`), DR-023 (generation-gated compute), DR-005 (reactive signals).

## Context

DR-023 gated `LayoutEngine::compute` on `Cx::mutation_generation()` — 7 LOC, 7 ns on the static S1 path. It shipped as `a58355f`.

When the vertical-slice example (`examples/vertical-slice`) landed, the generation-only cache produced stale layout under reactive text changes:

- `state.status.set("Run 1 queued for …")` writes a `Signal<String>` directly.
- Signal writes go through `sigil-reactive`'s runtime, not `Cx`. They do **not** call `Cx::mark_mutated()` — that is reserved for tree-shape mutations (`cx.add`, `cx.remove_child`, etc.) per DR-015.
- `mutation_generation()` therefore stayed constant across frames where the status text changed.
- The cached key matched; `compute()` short-circuited; Taffy kept its previous text measurement; downstream layout was wrong.

The hit-test invariant from DR-023 ("`LayoutEngine::generation()` only bumps on real re-run") was preserved, but layout itself was stale. The Tier 3 performance wins stood; correctness regressed.

## Decision

Extend the cache key from a single `u64` generation counter to a composite struct:

```rust
pub(crate) struct LayoutCacheKey {
    pub(crate) cx_instance: u64,     // distinguishes two fresh Cx instances
    pub(crate) root: NodeId,
    pub(crate) available: AvailableKey,  // bit-exact available size
    pub(crate) cx_generation: u64,   // tree-shape mutations (DR-023)
    pub(crate) input_signature: u64, // O(n) hash of layout-affecting inputs
}
```

`input_signature` walks the subtree once and feeds an `ahash::AHasher` with:

- `NodeId` and `NodeType` discriminant of each visited node.
- `TextContent` — string bytes for `Static`, `signal.peek()` bytes for `Reactive`.
- Layout-affecting `StyleAtom` variants from each node's `StyleBinding` (padding/margin/gap, width/height/min/max, display, flex, font-size/weight/line-height). Non-layout atoms (background, color, border) are intentionally excluded so paint-only reactive changes don't force recompute.

Lives in `crates/sigil-layout/src/cache_key.rs`.

### Fast-path ordering

`LayoutEngine::compute` checks the cheap fields first. If `cx_instance`, `root`, `available`, or `cx_generation` mismatch, the tree walk is skipped entirely and we go straight to a full recompute — so cold frames and tree-mutation frames never pay the content-hash cost. The walk runs only when the cheap fields all match, as a second-stage verification that reactive inputs are unchanged.

This means the DR-023 7 ns S1 path becomes "O(n) walk producing the same hash" on reactive apps, and stays close to O(1) on apps with no reactive bindings. Vertical-slice is ~40 nodes; the walk is ≪ 1 µs.

### `Cx::instance_id`

`Cx` gains one new field (`instance_id: u64`, monotonic `AtomicU64`) and one new method (`pub const fn instance_id(&self) -> u64`). This lets two fresh `Cx` instances reusing the same slotmap keys be distinguished by the cache. No other `Cx` semantics change.

## Rationale

**Why the tree walk is necessary.** `sigil-reactive` is intentionally decoupled from `Cx`. Plumbing every `Signal::set` through `Cx::mark_mutated` would (a) couple two crates that DR-005 keeps apart, (b) bump `mutation_generation()` for paint-only signal writes that don't affect layout, and (c) require a Signal → Cx back-reference that doesn't exist. Hashing the subtree is the cheapest way to observe reactive state from inside `LayoutEngine` without teaching `Cx` about signals.

**Why `ahash` not `DefaultHasher`.** `sigil-layout` already depends on `ahash` for its node maps. Std `DefaultHasher` has no stability guarantee across Rust versions and no collision-resistance guarantee. Using the hasher that's already in the crate is consistent and slightly faster.

**Why scope the style-atom hash.** Hashing every atom would invalidate layout on hover/press color changes — the vertical-slice would recompute layout on every pointer-move event. The `is_layout_affecting_atom` filter is the smallest set of variants that actually feed Taffy.

**Why `cx_instance` instead of reusing `Owner::id`.** Cache owners need to distinguish two fresh `Cx` instances whose slotmap `NodeId`s happen to collide (e.g. test harnesses that create a new `Cx` per test and reuse the engine). `Owner::id` is tied to the reactive runtime's scope and isn't the right identity for this.

## Consequences

### What changes from DR-023

- **Cache-key shape** — single `u64` becomes a 5-field struct.
- **Halt condition 5 from the Phase 10 plan ("No `Cx` modifications")** — lifted. `Cx::instance_id` is a new public API. No other `Cx` semantics change.
- **Halt condition 6 ("≤7 LOC in `engine.rs`")** — lifted. The cache-key struct and walk live in `cache_key.rs`; `engine.rs` grows by ~30 LOC for the fast-path dispatch.
- **S1 performance characterization** — "effectively free, 7 ns" becomes "O(k) where k is the layout-affecting-style-atom count in the subtree." For the vertical-slice (~40 nodes) that's sub-microsecond. Static-tree apps with no reactive bindings and no visited atoms still hit the cheap-fields fast path without walking.

### What stays from DR-023

- **Variant B strategy** — we still gate on `Cx::mutation_generation()` first; content-hash is an additional gate, not a replacement. Variant C (dirty propagation, `Cx::mutation_log`) remains parked.
- **`LayoutEngine::generation()` bump rules** — only bumps on real recompute. Downstream hit-test / focus-ring cache keys unchanged.
- **`compute()` signature, error semantics, and the Tier 3 budget claim for tree mutations** — unchanged.

### Test surface

`crates/sigil-layout/tests/incremental.rs` grows with four new tests exercising the new invalidation channels:

- `available_size_change_invalidates_cache`
- `computing_different_root_in_same_cx_invalidates_cache`
- `reusing_engine_with_fresh_cx_invalidates_cache`
- `reactive_text_change_invalidates_cache`
- `reactive_layout_style_change_invalidates_cache`

The six tests DR-023 mandated still pass unchanged.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Route `Signal::set` through `Cx::mark_mutated` | Couples `sigil-reactive` to `sigil-core`; bumps generation for paint-only signals; requires back-references DR-005 explicitly avoids. |
| Hash every `StyleAtom` variant | Hover/press color changes would invalidate layout. |
| Persist the paragraph layout across frames keyed on signal value | Doesn't solve the problem — Taffy owns measurement caches, not text shaping. |
| Introduce a layout-specific dirty bit on reactive signals | Cross-crate coupling with Phase 09 reactive recomposition not yet landed. |

## Kill criteria

Revisit if:

- A real app's profile shows `layout_input_signature` dominating frame time at ≥1 k nodes — then move to per-subtree incremental hashing or adopt Variant C.
- Phase 09 reactive recomposition introduces a cleaner "which signals affect layout" signal that `LayoutEngine` can observe directly, making the content-hash redundant.
- Style-atom enum gains enough variants that `is_layout_affecting_atom` becomes unmaintainable — then split the enum into `LayoutAtom` vs `PaintAtom`.

## References

- DR-023 — generation-gated compute (superseded in part by this DR).
- DR-015 — `Cx::mark_mutated` semantics that this DR preserves.
- `crates/sigil-layout/src/cache_key.rs` — the shipped implementation.
- `crates/sigil-layout/tests/incremental.rs` — test coverage.
- Commit `9436dfa` — "feat(vertical-slice): interactive dashboard proof + text-in-scene wiring".
