# DR-001 — Renderer: wgpu upstream, HAL-wrapped

**Status:** Accepted
**Date:** 2026-04-14

## Context

We need a cross-platform GPU abstraction for the paint pipeline. The
options are:

1. Depend on `wgpu` directly.
2. Fork `wgpu`.
3. Build our own GPU abstraction (as Zed did with Blade).

Evidence:

- `wgpu` is the de-facto standard Rust GPU crate. Bevy, Blitz, Floem
  (optional Vello backend), and Dioxus-Desktop all consume it upstream.
- Zed **migrated from their own Blade renderer to wgpu in 2025** because
  owning the GPU abstraction layer cost more than it bought at their team
  size. This is the clearest public evidence that forking or rolling our
  own is a trap at small team sizes.
- `wgpu` ships breaking releases on a ~3-month cadence. Consumers pay a
  migration tax at each major.

## Decision

Depend on `wgpu` upstream. Wrap every `wgpu` touchpoint behind a
`Renderer` trait in `sigil-hal`. Downstream crates name the trait, never
`wgpu` directly.

The HAL insulates us from breaking releases, gives us a test seam
(CPU/headless renderer for unit tests), and keeps the door open to
alternative backends (software rasteriser, Vello) without touching
consumer code.

## Consequences

- Every `wgpu`-typed value lives behind `Renderer` trait methods. No
  `wgpu::Surface` in `sigil-core` or higher.
- We pay a small abstraction cost (trait indirection, type erasure where
  needed). The HAL target is ~2k LoC across all six traits, so the cost
  is bounded.
- Migrations are confined to `sigil-renderer`. When `wgpu` ships a breaking
  release, only that crate changes.

## Kill criteria

Reconsider if:

- `wgpu` is abandoned or ownership changes in a way that threatens
  stability (the project is currently healthy).
- A measured bottleneck in the paint pipeline is traced to the HAL
  indirection itself (not the underlying GPU work).
- A dominant Rust desktop framework ships on a different backend and
  interop pressure makes `wgpu` a liability.

## References

- [wgpu](https://github.com/gfx-rs/wgpu)
- [Zed's "videogame" blog post — Blade to wgpu migration](https://zed.dev/blog/videogame)
