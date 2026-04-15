# DR-000 — Why not fork Floem

**Status:** Accepted
**Date:** 2026-04-14

## Context

Floem v0.2.0 (November 2025) ships the Rust UI foundation we need: signals,
Taffy layout, an optional Vello renderer, keyframe + spring animations, and
an ECS-based view implementation. Lapce is a live production consumer. On
paper, forking Floem would skip months of kernel work.

Research findings (April 2026):

- Floem does **not** ship AccessKit integration (issue #8, open since 2022).
  The April 2025 Rust GUI survey reports "Windows Narrator can't see any of
  this text."
- Floem does **not** ship working IME. Text editor stable-cursor issue
  (#1030) and IME delete support (#1024) are open.
- Floem does **not** ship documentation for its widget set (#616) or an
  externally-maintained component library (#1064).
- Floem v0.2.0 introduced an ECS-based view implementation — maximum churn
  surface in the newest code.
- WindowResized triggers full view-tree traversal (#1016) — a performance
  regression rooted in how coarse events route through the view tree.
- Signal ergonomics pain is documented in Floem's own discussion #435.
- Floem constrains containers to ~16 widgets due to tuple generics, a type
  system consequence that leaks into every consumer.
- Floem's built-in text editor is inherited from Lapce and is tuned for
  Lapce's needs. It does not use Parley.
- Lapce drives Floem's roadmap. Our roadmap needs to drive ours.

## Decision

We do not fork Floem. We build a new library with its own kernel.

Floem is a **primary reference** — we will read its source aggressively
for:

- winit 0.30+ `ApplicationHandler` + wgpu/Vello integration patterns
- Taffy adapter shape
- Animation system (keyframes + spring physics)
- ECS view-impl tradeoffs (learn what went wrong and right)
- The `Class` system pitfalls to avoid in our `classes!` macro

We borrow patterns, never code. Where we pick the same approach as Floem,
we cite Floem in the relevant decision record.

## Consequences

- We own every layer above the foundation crates (winit, wgpu, Taffy,
  Parley, AccessKit). This is a multi-year project.
- We can shape the node type, owner model, semantic identity, text engine,
  and style system around our priorities (accessibility, IME, component
  ergonomics) without negotiating with Lapce's roadmap.
- We lose the optics of "building on a known-good base." We gain a
  load-bearing kernel designed for our goals.

## Kill criteria

This decision would be reconsidered only if:

1. Floem announces a complete AccessKit integration, working IME on all
   three desktop OSes, and a stable external component library with
   documentation, **and**
2. Floem's maintainership model changes to give external contributors
   roadmap influence comparable to Lapce's, **and**
3. Our own kernel slice (Phase 1) has not yet landed.

If all three hold, we revisit. Otherwise this record stands.

## References

- [Floem releases](https://github.com/lapce/floem/releases)
- [Floem AccessKit issue #8](https://github.com/lapce/floem/issues/8)
- [Floem WindowResized regression #1016](https://github.com/lapce/floem/issues/1016)
- [Floem signal ergonomics discussion #435](https://github.com/lapce/floem/discussions/435)
- [2025 Rust GUI survey — boringcactus.com](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
