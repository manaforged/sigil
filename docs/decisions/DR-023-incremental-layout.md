# DR-023 — Incremental layout via generation gating

**Status:** Accepted, cache-key shape superseded by [DR-024](DR-024-layout-cache-key-content-hash.md) — Variant B strategy retained, halt conditions 5 and 6 lifted.
**Date:** 2026-04-17
**Closes:** master-roadmap "Phase 10 incremental layout"
**Depends on:** Spike 1 ([`docs/spikes/01-layout-invalidation.md`](../spikes/01-layout-invalidation.md)), DR-015 (`Cx::mark_mutated`), DR-009 (Tier 3 budgets)

## Context

Before this DR, `LayoutEngine::compute` re-synced the entire `sigil-kernel` tree to Taffy and re-computed layout on every call, regardless of whether anything changed. Spike 1 measured this on a 10 101-node tree against three variants:

| Scenario | Variant A (current) | **Variant B (this DR)** | Variant C (parked) | Tier 3 budget |
|---|---:|---:|---:|---:|
| S1 static (no mutation) | 21.1 ms ❌ | **7.0 ns ✓** | 7.2 ns ✓ | 16 ms |
| S2 one-mutation | 19.7 ms ❌ | **8.4 ms ✓** | 0.44 ms ✓ | 16 ms |
| S3 structural | 18.5 ms ❌ | **11.9 ms ✓** | 21.9 ms ❌ | 16 ms |

Variant A — the unchanged engine — misses the 16 ms Tier 3 frame budget on every scenario. Phase 10 was therefore not optional.

## Decision

Adopt **Variant B — generation gating**.

`LayoutEngine` caches the `Cx::mutation_generation()` value from its last successful `compute()`. Subsequent `compute()` calls with an unchanged generation short-circuit immediately, doing zero work: no Taffy sync, no Taffy layout, no `LayoutResult` refresh, and no `LayoutEngine::generation()` bump.

Production code change is ~7 lines in `crates/sigil/layout/src/engine.rs`. No `Cx` modifications. No `LayoutEngine` API changes — same `compute()` signature, same external contract.

## Rationale

**Why B over A.** A is over budget; the framework cannot ship at 10k nodes without invalidation tracking.

**Why B over C.** C is dramatically faster on S2 (single-node mutation: 443 µs vs B's 8.4 ms) but regresses S3 over budget (21.9 ms vs B's 11.9 ms) and adds 115 LOC of mutation-log scaffolding in `Cx`. The S2 win does not justify the S3 regression — structural changes are more impactful to user-perceived smoothness than per-frame mutation latency at the 8 ms scale. The mutation-log API C requires also leaks layout-implementation concerns into `Cx`, which DR-015 and the master-roadmap design explicitly avoid.

**Why generation gating preserves the hit-test invariant.** `LayoutEngine::generation()` only bumps when layout actually re-ran. Cached frames return immediately without bumping. Downstream consumers (hit-test cache, focus ring, style apply) keep their existing `(cx_generation, layout_generation)` cache keys without spurious invalidation.

## Consequences

### Performance characteristics post-DR

- **Static frames (no `Cx` mutation)** — effectively free. 7 ns per `compute()` call.
- **One-mutation frames** — full layout recompute (~8 ms at 10k nodes). Acceptable for typical reactive UIs that touch 1–10 nodes per frame.
- **Animation-heavy workloads** — at 60 fps with per-frame mutations, layout consumes 60 × 8 ms = 480 ms/sec, leaving ~520 ms/sec for everything else. This is the ceiling. Apps that exceed it need C-style dirty propagation (parked as Phase 10b).
- **Structural changes** — full recompute (~12 ms at 10k nodes). Unchanged from Variant A's behaviour, well under budget.

### What does NOT change

- `LayoutEngine::compute(cx, root, available)` signature, return type, error semantics.
- `LayoutEngine::generation()` bumping rules — still only bumps on real re-run.
- `Cx::mutation_generation()` and `Cx::mark_mutated()` semantics from DR-015.
- All existing layout tests pass without modification.

### Variant C parked

Variant C (Taffy-dirty propagation + `Cx::mutation_log`) is **explicitly parked** for a possible future "Phase 10b" if real animation workloads expose B's per-mutation cost as a bottleneck. The conditions to reopen:

- A real app demonstrates >60 fps mutation rate with measurable jank attributable to layout.
- The S3 regression in C's spike implementation can be eliminated (Spike 1's C had a structural-change fall-back that paid full sync; a cleaner implementation could tag structural mutations differently).
- A `Cx::mutation_log` API can be designed without leaking layout-implementation concerns into `sigil-kernel`.

Until those conditions hold, ship B and don't touch C.

## Alternatives rejected

| Variant | Decision | Reason |
|---|---|---|
| A — control | Rejected | Over Tier 3 budget on every scenario at 10k nodes. |
| **B — generation gating** | **Accepted** | Passes every scenario in 7 LOC. Preserves DR-015 invariants. |
| C — Taffy dirty propagation | Parked | S3 regression + 115 LOC complexity + leaks into `Cx`. Reopen if animation workloads expose B's S2 ceiling. |

## References

- [`docs/spikes/01-layout-invalidation.md`](../spikes/01-layout-invalidation.md) — Spike 1 measurements + recommendation.
- [`docs/spikes/01-layout-invalidation-brief.md`](../spikes/01-layout-invalidation-brief.md) — the the reviewer brief that produced the spike.
- [`docs/plans/phase-10-incremental-layout.md`](../plans/phase-10-incremental-layout.md) — execution playbook.
- Spike branches (delete after this DR ships): `spike-01-rig` (`759d205`), `spike-01-A` (`f302f81`), `spike-01-B` (`6eeb93a`), `spike-01-C` (`7eef47e`).

## Kill criteria

Revisit if:

- A real app's profiling shows the 8 ms-per-mutation cost is the dominant frame-time consumer.
- Phase 09's reactive recomposition produces structural-change patterns that consistently push S3 over budget.
- Taffy gains a native incremental-layout API that supersedes both B and C.
