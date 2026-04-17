# DR-015 — Fragment semantics: flatten into parent

**Status:** Accepted
**Date:** 2026-04-16
**Closes:** DR-013 follow-up F-001
**Depends on:** DR-010 (`sigil-core` keystone), DR-011 (`sigil-layout` adapter)
**Unlocks:** Phase 08 (widgets)

## Context

`NodeType::Fragment` was introduced in Phase 03 as the tree-shaped analogue
of React.Fragment — a grouping node with no box of its own. DR-013 logged
this as follow-up **F-001** and noted that the semantics needed to be
pinned down before component primitives (widgets) started composing
fragments heavily.

The repo-local plans (Phases 04–07) chose an implementation without
recording a DR. Phase 08 (widgets) is the first consumer that will rely
on fragment semantics at scale, so the decision needs to be written down
before the next layer is built on top of it.

Three shapes were considered:

1. **Flatten into parent.** The fragment has no Taffy node; its children
   are spliced in as siblings of the fragment's parent. The fragment
   node still exists in the `sigil-core` tree — only the layout engine
   is blind to it.
2. **Transparent adapter-only node.** The fragment gets a zero-sized,
   zero-padded, `display: contents` Taffy node. Layout treats it as a
   pass-through but it still exists in the Taffy tree.
3. **Different core representation.** `Fragment` is not a `NodeType` at
   all. Sibling grouping becomes a builder concept (`NodeBuilder::group`)
   that adds children directly to the parent at `cx.add` time.

## Decision

Adopt **option 1 — flatten into parent.** This codifies the implementation
that shipped across Phases 04–07.

- `sigil-layout`: `sync_subtree` short-circuits on `NodeType::Fragment`
  and returns the fragment's children's Taffy IDs in place. The fragment
  has no Taffy node, no `LayoutResult`, no geometry.
- `sigil-renderer`: `emit_scene` walks through fragments and emits nothing
  for them.
- `sigil-events`: fragments are transparent to the router. Hit-test never
  lands on a fragment (no geometry), but capture/bubble walks the
  `sigil-core` tree, so `.on_pointer_down(…)` attached to a fragment
  fires for its subtree as expected.
- `sigil-core`: the fragment remains a real node with a `NodeId`, an
  `Owner`, and an extension bag. Reactivity, component ownership, and
  extension attachment work uniformly across fragments and containers.

## Rationale

**Why not option 2 (adapter-only Taffy node).** Taffy's `display: contents`
support is partial and carries per-node overhead (still a Taffy slot,
still a style struct, still a layout result slot). Layout is the hot path
the benches in DR-009 actually budget; paying Taffy cost for a node that
contributes nothing to geometry is pure loss. Option 1 skips Taffy entirely.

**Why not option 3 (builder-only grouping).** Fragments must survive past
`cx.add`. Conditional rendering (`if cond { Button } else { Link }`),
list reconciliation, and reactive swaps all need a stable parent handle
to reattach children under — erasing the fragment at build time destroys
that handle. Option 3 also kills `Owner`-keyed disposal for a fragment's
subtree, which DR-005 relies on.

**Why option 1 is right.** It matches the React mental model users already
carry, keeps fragment cost to one node-slot + one children Vec, and
preserves every cross-cutting concern (ownership, reactivity, events) by
routing through the node tree rather than the layout tree. Layout and
rendering opt out, which is exactly the point of a fragment.

## Consequences

### Load-bearing invariants

- `Fragment` nodes **never** have a `LayoutResult`. Consumers of
  `cx.get(id).and_then(result_of)` must already tolerate `None`; the
  fragment is one of the cases that produces it.
- `Fragment` nodes **never** produce `SceneItem` output. Renderer
  back-ends that enumerate nodes directly (bypassing `emit_scene`) must
  match on `NodeType::Fragment` and skip.
- Events **do** walk through fragments. Handlers attached to a fragment
  are active; stop-propagation, capture ordering, and pointer capture
  all behave as if the fragment were a container with an infinite hit
  region that is simply never hit directly.

### Footguns to guard

Two patterns are syntactically legal but semantically meaningless:

- **`fragment().class("bg-accent")`** — style applies to a node with no
  paint box. The atoms are attached, merged, and resolved, and then
  nothing renders.
- **`fragment().focusable(true)`** — the fragment enters the tab ring but
  has no geometry for the focus indicator to draw against.

Both are caught with `debug_assert!` in the respective builder extension
points (`NodeStyleExt::class`, `NodeBuilder::focusable`). Release builds
no-op silently rather than panic, because the worst-case production
outcome is a missing style/focus ring, not a crash.

### Out of scope

- **`display: contents`-style visual layouts.** If a future layout
  primitive genuinely needs "participates in layout but has no box" (e.g.
  CSS `display: contents` for accessibility-only grouping with visual
  flow), that is a new `NodeType`, not a reinterpretation of `Fragment`.
- **Fragment-scoped styles.** A fragment cannot carry a scoped style
  context that applies to its children only; class inheritance is per-node.
  Widgets that want scoped theming should use a `Container` with
  `LayoutStyle::contents()` once that primitive exists, or a theme
  provider extension.

## Alternatives rejected (summary)

| Option | Decision | Reason |
|--------|----------|--------|
| (1) Flatten into parent | **Accepted** | Matches React model, zero Taffy overhead, preserves ownership/events. |
| (2) `display: contents` adapter node | Rejected | Per-fragment Taffy cost with no upside. |
| (3) Builder-only grouping | Rejected | Loses stable parent handle for reactive swaps and `Owner`-keyed disposal. |

## References

- [`docs/decisions/DR-013-roadmap-reconciliation.md`](DR-013-roadmap-reconciliation.md)
  — F-001 logged.
- [`crates/sigil-core/src/node.rs`](../../crates/sigil-core/src/node.rs)
  — `NodeType::Fragment` definition + doc comment.
- [`crates/sigil-layout/src/engine.rs`](../../crates/sigil-layout/src/engine.rs)
  — `sync_subtree` flattening.
- [`crates/sigil-layout/tests/fixture_suite.rs`](../../crates/sigil-layout/tests/fixture_suite.rs)
  — `fragment_children_treated_as_siblings_of_parent` regression test.
- [`crates/sigil-renderer/src/scene.rs`](../../crates/sigil-renderer/src/scene.rs)
  — scene emission no-op.

## Kill Criteria

Revisit if:

- A consumer legitimately needs fragment-scoped layout (not just logical
  grouping) and the "new `NodeType`" escape hatch becomes unergonomic.
- Taffy gains first-class `display: contents` with equal or lower cost
  than the current flatten pass, at which point option 2 becomes
  strictly better than option 1 on the "debugger can see the node in
  the layout tree" axis.
- Profiling shows the `sync_subtree` recursion through deep fragment
  chains becomes a hot path worth optimizing with a Taffy pass-through
  node instead.
