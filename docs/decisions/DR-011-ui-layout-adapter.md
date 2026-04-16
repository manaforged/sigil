# DR-011 - sigil-layout adapter

**Status:** Accepted
**Date:** 2026-04-15

## Context

Phase 3 landed `sigil-core`: stable node IDs, accessible-by-default
semantics, typed extensions, component owners, and hand-written
builders. The core tree still had no computed geometry. Renderers,
hit-testing, and accessibility bounds need a local layout result to
read from each node without borrowing a global engine.

DR-003 chose upstream Taffy behind the HAL instead of a fork. The
repo-local Phase 4 plan at
[`docs/plans/phase-04-sigil-layout-adapter.md`](../plans/phase-04-sigil-layout-adapter.md)
proposed the first load-bearing adapter. An adversarial pass found two
important adjustments:

- The plan named Taffy 0.10.0 as current, but crates.io reports 0.10.1
  as the latest 0.10 patch. The workspace pins the 0.10 line with
  `taffy = "0.10"` rather than an exact patch.
- The repo already had Phase 3 in flight as uncommitted work. Phase 4
  must layer on top of that API without reshaping `sigil-core`.

## Decision

Add `sigil-layout` as a headless Taffy-backed adapter.

- `LayoutEngine` owns one `TaffyTree<LayoutContext>` and bidirectional
  maps between `sigil_core::NodeId` and Taffy node IDs.
- `LayoutIntent` is a thin newtype over `taffy::Style`, attached to
  nodes through `NodeExtensions`.
- `LayoutResult` is attached back to nodes after compute as a read-only
  projection and stores a parent-relative `sigil_hal::Rect` plus content
  size.
- Text leaves use a placeholder `DefaultTextMeasure`: static characters
  are `8.0` logical pixels wide and one line is `16.0` logical pixels
  high. Reactive text returns zero width until `ui-text` owns
  read-on-compute behavior.
- `TextMeasure` is the dependency-injection point for `ui-text`.
- `sigil_hal::Layout` is now a concrete trait with `Intent`, `Result`,
  `upsert`, `set_children`, `remove`, `compute`, and `result`.
- The phase stays headless. `examples/hello-layout` computes and prints
  rectangles; renderer integration remains out of scope.

## Consequences

`sigil-layout` is the first downstream consumer of `NodeExtensions` and the
first HAL trait to graduate from placeholder to load-bearing. Taffy
stays private to the adapter except for `LayoutIntent`, which exposes a
controlled style-builder surface and a deliberate raw-style escape hatch
for uncommon CSS features.

`LayoutEngine::result` is authoritative for direct HAL-style reads after
the most recent successful compute. The node extension is the local
read-path projection for tree consumers. `compute` refreshes projections
for the computed root and clears stale projections from nodes that were
part of the previous compute but are no longer under that root.

The adapter mirrors `sigil-core::Fragment` as a normal Taffy node for now
because Taffy has no `display: contents` equivalent. That is acceptable
for the Phase 4 headless proof, but fragment transparency is tracked by
DR-013 follow-up F-001 and must be resolved before fragments are used
heavily by primitives.

The fixture suite is Rust-native rather than XML/WPT-backed. It starts
with small flex and padding regressions and can grow as DR-003's open
Taffy bug set becomes relevant to real primitives.

## Initial Bench Measurements

First local smoke run, measured with
`cargo bench -p sigil-layout --bench layout_ops -- --warm-up-time 1 --measurement-time 3 --sample-size 30`
on the Windows development machine:

| Operation | Measurement | Phase 4 provisional budget |
| --- | ---: | ---: |
| `compute_single_node` | 348.20 ns to 380.06 ns | < 5 us |
| `compute_flex_row_100_children` | 48.569 us to 51.892 us | < 200 us |
| `compute_tree_100` | 45.658 us to 48.263 us | not yet budgeted |
| `sync_tree_1000_nodes` | 138.32 us to 143.73 us | < 1 ms |
| `gc_after_remove_100` | 26.140 us to 33.572 us | < 50 us |

These are local criterion smoke numbers, not cross-machine baselines.
They establish the defended Phase 4 floor until the CI baseline machinery
from DR-009 is wired for `sigil-layout`.

## Kill Criteria

Revisit this decision if:

- Taffy API churn breaks the adapter shape often enough that the newtype
  no longer buys stability.
- Measured layout hot paths allocate or scale poorly in a real consumer
  after sync/compute/gc are profiled.
- A real primitive or renderer needs fragment semantics that cannot be
  represented by the current mirror.
- A measured consumer needs a non-Taffy backend. In that case, build a
  second adapter behind `sigil_hal::Layout` rather than leaking Taffy into
  callers.

## References

- [DR-003 - Layout: Taffy upstream behind HAL](./DR-003-layout-taffy.md)
- [DR-009 - Performance Discipline](./DR-009-performance-discipline.md)
- [DR-010 - sigil-core keystone crate](./DR-010-sigil-core-keystone.md)
- [DR-013 - Roadmap reconciliation and plan traceability](./DR-013-roadmap-reconciliation.md)
