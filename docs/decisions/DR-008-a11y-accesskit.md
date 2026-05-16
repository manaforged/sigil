# DR-008 — Accessibility: AccessKit `TreeUpdate` stream from Phase 1

**Status:** Accepted
**Date:** 2026-04-14

## Context

Accessibility is the perennial "next quarter" task in every Rust UI
framework. Floem's AccessKit integration issue #8 has been open since
2022. GPUI has no AccessKit integration. Xilem/Masonry is the closest to
real — the AccessKit author contributes directly — and the April 2025
Rust GUI survey still reports "missing glyph rendering in intermediate
IME states."

The one framework that ships working screen reader support is **Slint**
("Narrator works perfectly" per the 2025 survey). Slint's pattern:
declarative semantics alongside the visual tree, compiled to AccessKit
`TreeUpdate`s.

Research findings on AccessKit integration patterns:

- AccessKit is a **parallel tree** published via `TreeUpdate`, not an
  aspect of the consumer's node type.
- The core types are `Tree` (global), `Node` (a single accessible
  object), and `TreeUpdate` (atomic change). Consumers push
  `TreeUpdate`s to a platform adapter (Windows UI Automation, macOS
  NSAccessibility, Linux AT-SPI2).
- Bidirectional action dispatch via `ActionHandler`, `ActivationHandler`,
  `DeactivationHandler` traits.
- The `tokio` feature avoids the zbus blocking-API panic on Linux when
  AccessKit is called from an async runtime.

Retrofitting accessibility after primitives exist means touching every
component. Floem, GPUI, and Xilem all demonstrate how hard this is in
practice.

## Decision

Accessibility is a **Phase 1 concern**, not a Phase 9 concern.

The platform adapter completion defers — at Phase 1 we wire AccessKit
at minimal coverage (one window, one button, one text input). But the
**semantic identity** that feeds `TreeUpdate`s lives on the node model
from the first commit:

- Every `NodeId` in `sigil-kernel` is also the `accesskit::NodeId`. One
  identity, one tree.
- Every node carries a `Semantics` struct:
  - `role` — `Role::Button`, `Role::TextInput`, etc.
  - `name_source` — `Static(String) | Derived(Signal<String>) | LabelledBy(NodeId)`
  - `description_source` — parallel shape
  - `actions: Vec<Action>` — invoke, toggle, increment, expand, etc.
  - `keyboard_shortcut: Option<Shortcut>`
  - `focusable: bool`
  - `tab_index: Option<i32>`
- The `ui-a11y` crate derives an incremental `TreeUpdate` stream from
  node mutations, keyed by the stable `NodeId`.
- Action dispatch is authored by whoever writes the node, not bolted on
  later.
- Every primitive in the catalog has a **mandatory a11y test** using
  AccessKit's test harness before it can merge.
- `accesskit` is consumed with the `tokio` feature to avoid the zbus
  panic.

Platform adapter coverage grows from Phase 1 onward as primitives land.
Phase 9 completes platform adapter surface coverage and adds CI screen
reader smoke tests (NVDA on Windows, VoiceOver on macOS, Orca on Linux).
**Phase 9 is not where accessibility starts — it is where platform
adapter coverage is completed and test surface is hardened.**

## Consequences

- Every primitive has non-trivial boilerplate for `Semantics`. The
  component builder API makes this the default path, not an opt-in.
- Stable `NodeId` must be generated at node creation time and preserved
  across re-layouts. This shapes the node type from day one — there is
  no retrofitting it later without touching every component.
- Focus order is a product of how nodes are composed. The focus manager
  derives `TreeUpdate` focus changes from the composed tree.
- The platform adapter is stubbed on platforms we have not wired yet,
  so consumers developing on an unfinished platform still get a
  semantic tree for programmatic testing.

## Kill criteria

This decision does not have meaningful kill criteria. Accessibility is
a non-negotiable requirement for a 2026 desktop UI library. The only
path that changes is whether AccessKit remains the right abstraction —
if a better cross-platform a11y library appears, we replace the platform
adapter while keeping the semantic tree shape.

## References

- [AccessKit](https://github.com/AccessKit/accesskit)
- [AccessKit docs.rs](https://docs.rs/accesskit/latest/accesskit/)
- [Floem issue #8 — AccessKit integration](https://github.com/lapce/floem/issues/8)
- [Zed discussion #6576 — AccessKit in GPUI](https://github.com/zed-industries/zed/discussions/6576)
- [2025 Rust GUI survey](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
