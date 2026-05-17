---
id: DR-041
status: accepted
date: 2026-05-16
covers: P0 sub-tasks — P0.1 tree plumbing, P0.2 block, P0.3 flex, P0.4 parallel measure, P0.5 taffy swap-out, P0.6 perf gates, P0.7 docs
depends_on: [DR-040, DR-003, DR-011, DR-024, DR-034]
relates: docs/plans/2026-05-16-sigil-r15-dispatch-plan.md
---

# DR-041 — Owned Layout Engine (P0)

**Status:** Accepted
**Date:** 2026-05-16
**Covers:** P0 sub-tasks P0.1 (tree plumbing), P0.2 (block layout), P0.3 (flex layout), P0.4 (parallel measure), P0.5 (taffy swap-out), P0.6 (perf gates), P0.7 (docs)
**Depends on:** DR-040 (sigil thesis lock), DR-003 (current taffy adapter rationale), DR-011 (ui-layout adapter shape), DR-024 (layout cache key contract), DR-034 (layer-scope semantics)

---

## Context

`sigil-layout` ships today as a thin adapter over `taffy` (~1345 LoC across 13 files). The Cargo description reads: "Taffy-backed layout adapter for Sigil." Widgets call `LayoutBuilderExt::layout(LayoutStyle::flex_row())` (or block / column variants); the adapter translates `LayoutStyle` to taffy's `Style`, runs taffy's solver, writes back `LayoutResult` extensions onto the kernel's `Cx` tree.

DR-040 locks the architectural thesis: own the foundation. `taffy` is the kind of dep we own now or fight forever:

- Single-threaded solver. Cannot parallelize independent subtrees without forking.
- Doesn't see sigil's scene graph. Cannot cache against our invalidation model without an adapter that always lags.
- GPU-driven measure (X1) requires writing the measure pass against our own buffers, not taffy's CPU types.
- The dep philosophy from DR-040 bans "planned replacement" tenure. Either commit to owning or commit to renting forever — we will not rent layout forever.

**P0 swaps the taffy backend behind the existing facade.** The public API surface (`LayoutBuilderExt`, `LayoutStyle`, `LayoutEngine`, `LayoutResult`, `LayoutError`, `TextMeasure`, `TextEngineMeasure`) stays — widget call sites are untouched. Only the engine's internals change.

---

## Decision

Replace the taffy backend in `crates/layout/src/engine.rs` + `style_adapter.rs` with an owned engine. Land in 7 sub-tasks (P0.1–P0.7) per the strategic plan and the dispatch plan.

### Module plan

| Module | Disposition | Why |
|---|---|---|
| `engine.rs` | **REWRITE** in P0.1 — owned forest + measure-pass state machine | The taffy adapter's body becomes the owned engine. Public method names frozen: `new`, `with_text_measure`, `sync`, `compute`, `gc`, `result`, `generation`, `mirrored_node_count`. |
| `style_adapter.rs` | **DELETE** in P0.5 | No taffy types to adapt to. `sigil_style::StyleAtom` → owned `StyleData` lands inline in `intent.rs`. |
| `block.rs` | **ADD-NEW** stub in P0.0, body in P0.2 (W2.3) | CSS block subset: margin collapse, padding, width/height with `auto`, min/max clamping, absolute pinning, content-box vs border-box sizing. |
| `flex.rs` | **ADD-NEW** stub in P0.0, body in P0.3 (W2.4) | CSS flexbox subset: main/cross axis, basis, grow, shrink, wrap. Ported against taffy's flexbox test suite as the correctness oracle. |
| `parallel.rs` | **ADD-NEW** stub in P0.0, body in P0.4 (W2.5) | Rayon fan-out across `Position::Absolute` subtrees + layer scopes (DR-034). Top-level measure stays serial; fan-out only at boundaries. |
| `intent.rs` | **KEEP-PUBLIC** | `LayoutStyle` stays. ONE break: `LayoutStyle::with_taffy` → `LayoutStyle::raw` (per authoring-syntax doc: explicit escape hatches must be named `raw` / `unchecked` / `expert`). Grep confirms zero in-tree callers. |
| `builder_ext.rs` | **KEEP-PUBLIC unchanged** | Widget call sites are untouched. `LayoutBuilderExt::layout(...)` API surface frozen. |
| `prelude.rs` | **REWRITE** in P0.5 | Replace `taffy::prelude::*` re-exports with sigil-owned types under the same names (`AlignItems`, `Dimension`, `Display`, `FlexDirection`, etc.). Removed: `Style` (no widget imports it). |
| `cache_key.rs` | **KEEP-INTERNAL**, rewire | Cache-key shape per DR-024 unchanged. Body rewires from taffy invalidation channels to forest mutation channels. |
| `measure.rs` | **KEEP-PUBLIC**, REWRITE signature | `TextMeasure` API stays but swap `taffy::Size` / `AvailableSpace` for sigil-owned types. |
| `result.rs` | **KEEP-PUBLIC unchanged** | `LayoutResult` shape stays. |
| `error.rs` | **KEEP-PUBLIC**, REWRITE one variant | `LayoutError::TaffyInvalid` → `LayoutError::InvalidTree`. |
| `text_adapter.rs` | **KEEP-PUBLIC**, REWRITE signature | `TextEngineMeasure` API stays. Internal types swap. |
| `text_style.rs` | **KEEP-PUBLIC**, REWRITE signature | `LayoutContext`, `measure_text`, `apply_text_binding`, `theme_for` keep names; internal types swap. G1 retunes the text path; P0 keeps it functional. |
| `adapter.rs` | **KEEP-INTERNAL**, REWRITE body | `impl Layout for LayoutEngine` body rewritten against owned engine. Trait surface frozen. |

### API trait sketch (new internal types — P0.1 lands the real shape; P0.2/P0.3/P0.4 land the bodies)

```rust
// crates/layout/src/engine.rs — landing in P0.1 (W2.2):

pub(crate) struct LayoutForest {
    nodes: Vec<LayoutNode>,
    boxes: Vec<ComputedBox>,
    free: Vec<LocalId>,
}

pub(crate) struct MeasureScope<'a> {
    forest: &'a LayoutForest,
    text: &'a dyn TextEngineMeasure,
    available: AvailableSpace,
}

pub(crate) enum LayoutPass { Measure, Arrange }

pub struct AvailableSpace { pub w: SpaceConstraint, pub h: SpaceConstraint }
pub enum SpaceConstraint { Definite(f32), MinContent, MaxContent }

// crates/layout/src/block.rs — already landed in P0.0 (W2.1):

pub(crate) struct BlockSolver<'a> { scope: MeasureScope<'a> }
impl<'a> BlockSolver<'a> {
    pub(crate) fn new(scope: MeasureScope<'a>) -> Self;
    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError>;
}

// crates/layout/src/flex.rs — already landed in P0.0 (W2.1):

pub(crate) struct FlexSolver<'a> { scope: MeasureScope<'a> }
impl<'a> FlexSolver<'a> {
    pub(crate) fn new(scope: MeasureScope<'a>) -> Self;
    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError>;
}

// crates/layout/src/parallel.rs — already landed in P0.0 (W2.1):

pub(crate) struct ParallelMeasure;
impl ParallelMeasure {
    pub(crate) fn collect(forest: &LayoutForest, root: NodeId) -> SmallVec<[ParallelBoundary; 8]>;
    pub(crate) fn measure(forest: &LayoutForest, scope: MeasureScope<'_>, boundaries: &[ParallelBoundary]) -> Result<(), LayoutError>;
}
pub(crate) struct ParallelBoundary { pub(crate) node: NodeId, pub(crate) kind: BoundaryKind }
pub(crate) enum BoundaryKind { Absolute, Layer }
```

All method names are **one word** per `a local document`. No `with_` prefix on builders. No trivial getters. Locals + struct fields are short canonical: `scope`, `node`, `kind`, `forest`, `boxes`, `style`, `text`, `nodes`, `free`, `children`. Reviewed against `api-ergonomics-reviewer` rules.

### Cargo dep removal plan (P0.5 — W2.6)

`taffy` stays in `crates/layout/Cargo.toml` until P0.5. Then drop in this order:

1. `crates/app/Cargo.toml` (uses `use taffy as _;` markers — drop dep)
2. `crates/events/Cargo.toml` (same)
3. `crates/renderer/Cargo.toml` (same)
4. `crates/sigil/Cargo.toml` (same)
5. `crates/text/Cargo.toml` (same)
6. **`crates/layout/Cargo.toml` last** (the holder)
7. Workspace `[workspace.dependencies]` taffy entry removed when no member uses it

### Perf gates (P0.6 — W2.7)

Per the strategic plan + dispatch plan:

- ≥ 1.5× speedup vs taffy on a 10k-node tree single-threaded
- ≥ 3× speedup vs taffy on a 100k-node tree at 4 cores (via parallel boundary fan-out)
- ≤ 1 buffer per `Position::Absolute` boundary (dhat snapshot)
- All 29+ existing integration suites green post-swap
- Pixel-test golden delta ≤ 0.5% on every widget

### Why now (citing DR-040)

DR-040 locked the dep philosophy: own what we'd ever replace; no "planned replacement" tenure. Layout is the case study. We pay the cost ONCE in P0, then never again.

---

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Keep taffy, add parallel fan-out layer on top | Taffy doesn't expose interior tree to let us coordinate; would be a fork in disguise. |
| Switch to a different rented engine (e.g., a `cosmic-text`-adjacent layout crate) | DR-040 bans this category — adds another forever-dep we don't differentiate on. |
| Defer P0 until G-substrate lands (G2 entity model, G4 DrawPhase) | Layout is independent of entity state and draw phase — they CALL layout, not the other way around. Sequencing P0 first lets every widget in Bucket B + C + D + E benefit immediately. |
| CSS Grid in P0 | Out of scope — block + flex covers 95% of widget layouts. Grid lands in X1 (GPU-driven measure extension) when a real consumer (the host engine editor multi-pane) demands it. |
| GPU-driven measure in P0 | X1's job. P0 = CPU correctness + parallel CPU fan-out. X1 adds GPU compute on top, once correctness is locked. |

---

## Consequences

**Positive:**

- Layout owned end-to-end. Future perf walls (GPU-driven measure, IDE-density 2000-node frames, 1M-row virtualized tables) are addressable without renting around them.
- API surface unchanged → widget code untouched.
- 6 crates lose a transitive dep (`taffy`); cargo-deny + audit surface shrinks.
- Free to add sigil-specific layout features (per-frame layout cache keyed by the host engine scene-graph generation, layer-scope-aware measure isolation, etc.) without convincing upstream.

**Negative:**

- 8-12 weeks of focused work (P0.1–P0.7).
- Need our own CSS-correctness oracle (port taffy's flex tests).
- Owning a layout engine is owning a layout engine — we now field bug reports on padding-with-min-content-and-absolute-positioning corner cases.

**Neutral:**

- `LayoutStyle::with_taffy` renamed to `LayoutStyle::raw`. Zero in-tree callers. Pre-1.0; external consumers don't exist yet. Document in P0.5 release notes.
- Prelude re-export name collisions (`AlignItems`, `Dimension`, etc.) resolved by removing taffy re-exports in the same P0.5 commit that adds the sigil-owned versions.

---

## Kill criteria

If at end of P0.3 (flex body, W2.4) the perf bench shows < 1× speedup vs taffy single-threaded, halt and reassess. Either:

- The owned engine has a fundamental algorithmic issue → fix it (don't ship slower).
- Or the workload doesn't benefit from owned layout → revert taffy, lift P0 to P-deferred, and re-plan around the X1 GPU path as the primary win.

The 1.5× single-threaded gate at P0.6 is the floor. Anything below means we built it wrong.

---

## Open questions

To be resolved in follow-up DRs (NOT in DR-041):

- **DR-042** — IME backend per OS (gates G7). Owns the bounds_for_range / replace_text_in_range platform plumbing.
- **DR-043** — Plugin host transport: in-process vs wasm-component (gates I5/I9 default decision).
- **DR-044+ TBD** — AI1/AI2/AI4/AI5 specifics as each Phase AI milestone dispatches.
- **CSS Grid timing** — explicit decision in X1 dispatch DR or its own DR-04N. Likely "ship X1 GPU measure first; revisit Grid after a real consumer requests it."

---

## References

- `a local document` — strategic plan §"P0 — Owned layout engine"
- `a local document` — dispatch plan §"P0 owned layout engine"
- DR-040 (sigil thesis lock)
- DR-003 (current taffy adapter rationale — historical)
- DR-011 (ui-layout adapter shape — historical)
- DR-024 (layout cache key contract)
- DR-034 (layer-scope semantics — informs `BoundaryKind::Layer`)
- `a local document` (the author-wide API doctrine; one-word methods, no `with_` prefix, configure-as-noun, etc.)
