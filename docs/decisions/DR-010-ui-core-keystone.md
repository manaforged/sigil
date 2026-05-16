# DR-010 - sigil-kernel keystone crate

**Status:** Accepted
**Date:** 2026-04-15

## Context

The foundation crates (`sigil-hal`, `sigil-reactive`, `sigil-async`,
`sigil-renderer`, and `sigil-shell`) establish the runtime floor, but
downstream UI work needs one shared node model. That model has to carry
identity, accessibility semantics, composition, reactive ownership, and
extension points from the first usable tree.

The repo-local Phase 3 plan at
[`docs/plans/phase-03-sigil-kernel-keystone.md`](../plans/phase-03-sigil-kernel-keystone.md)
reconciles the original kernel-slice plan with what has shipped. It
also locks the two standards for this phase: bulletproof internals and
ergonomic call sites. Shipping an internal-only arena without pleasant
builders would push usability debt forward; shipping only builders
without stable identity and disposal would make later layout, events,
and a11y fragile.

## Decision

Add `sigil-kernel` as the keystone crate.

- **Identity:** `NodeId` is a newtype over `slotmap::DefaultKey`.
  `NodeId::to_accesskit()` converts through `KeyData::as_ffi()` into
  AccessKit's `u64` node identity, preserving stable IDs across
  mutations and preventing aliasing after slot reuse.
- **Storage:** `Cx` owns a `SlotMap<DefaultKey, Node>`. Removed nodes
  become stale through slotmap's generation checks.
- **Composition:** `Component` is an open, one-shot trait:
  `fn build(self, cx: &mut Cx) -> NodeId`. `Cx::add` creates a
  per-component reactive owner and runs build inside it. `Cx::add_child`
  is the public composite-component path: it takes an explicit parent
  `NodeId`, creates a fresh per-component owner, and links the built
  root under the parent in the arena. `Cx::insert_raw` panics outside
  any reactive owner scope so primitive builders cannot silently leak
  component state onto the context root.
- **Element model:** `NodeKind` is a closed enum for hot-path core
  variants (`Container`, `Text`, `Fragment`). `NodeExtensions` is a
  typed bag for downstream crates to attach layout, style, event, text,
  or a11y-specific state without forcing a `sigil-kernel` release.
- **Accessibility:** every `Node` carries `Semantics`, never
  `Option<Semantics>`. Defaults use `Role::GenericContainer`, while
  built-in text defaults to `Role::Label`.
- **Builders:** hand-written builders and free functions ship now:
  `container()`, `fragment()`, and `text(...)`. No proc macros are part
  of this phase.
- **Public surface:** `sigil_kernel::prelude::*` exposes the application
  surface; slotmap keys and raw storage stay behind `NodeId` and `Cx`.

## Consequences

Downstream crates can build on a real tree without choosing their own
node IDs, owner disposal rules, or extension storage. Removing a subtree
disposes component owners in post-order, so signals, effects, and
cleanups created during build are torn down with the node tree.

The extension bag deliberately uses type IDs and boxed values. That is a
cold-path flexibility cost paid only by nodes with attached downstream
state. The core hot-path kind remains an enum so tree walks can match
exhaustively without dynamic downcasts.

`sigil-kernel` does not own layout, events, focus, styling, text shaping, or
platform a11y adapter wiring. Those crates attach their phase-specific
state through extensions and derive their own runtime structures from
the stable node tree.

## Initial Bench Measurements

First local smoke run, measured with `cargo bench -p sigil-kernel` on the
Windows development machine:

| Operation | Measurement | Phase 3 budget |
| --- | ---: | ---: |
| Add 1000 text nodes | 87.172 us to 89.805 us | <= 200 us |
| Remove 100-node subtree | 6.6166 us to 6.8849 us | <= 50 us |
| Walk 1000-node tree | 2.9574 us to 3.1744 us | <= 30 us |

These are criterion smoke numbers, not a cross-machine baseline. The
budget is the defended floor for Phase 3 until the CI baseline machinery
from DR-009 is wired for `sigil-kernel`.

## Kill Criteria

Revisit this decision if:

- AccessKit changes node identity away from `u64` in a way that breaks
  the slotmap FFI bridge.
- Real downstream crates need multiple same-type extension components on
  one node; the type-ID bag would need keyed slots rather than one value
  per type.
- Benchmarks show the boxed child-builder storage or extension bag
  allocation appears on a hot path that cannot be moved to setup time.
- The open `Component` trait begins encouraging hidden local state
  instead of explicit signal-backed state.

## References

- [DR-005 - Reactivity: Leptos 0.5+ owner tree](./DR-005-reactivity-owner-tree.md)
- [DR-008 - Accessibility: AccessKit TreeUpdate from Phase 1](./DR-008-a11y-accesskit.md)
- [DR-009 - Performance Discipline](./DR-009-performance-discipline.md)
- [DR-013 - Roadmap reconciliation and plan traceability](./DR-013-roadmap-reconciliation.md)
