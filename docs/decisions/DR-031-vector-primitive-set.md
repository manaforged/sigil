# DR-031 — Vector Primitive Set: R1–R13 Cut Decisions

**Status:** Accepted
**Date:** 2026-05-15
**Covers:** Renderer foundation phases R1–R13
**Depends on:** DR-020 (scene command buffer), DR-014 (Slug renderer)
**Unlocks:** R14 ergonomics pass (B-stream), Pack 4 widgets (D-stream)

## Context

The renderer foundation (R1–R13) was scoped against a full CSS-compatible
primitive list. Not all CSS paint primitives are equally valuable to a
game-adjacent desktop UI. The cut had to be made explicitly: ship the 80%
that every widget needs, defer the 15% that requires non-trivial new pipelines
or has no near-term consumer, skip the 5% that is actively harmful to the
current architecture.

The full candidate list came from CSS Backgrounds and Borders Level 4, CSS
Images Level 4, and GPUI's 2025 design research notes.

## Decision

Ship thirteen primitives across R1–R13. Defer four classes. Skip two.

### Shipped (R1–R13)

| Phase | Primitive | Key file |
|---|---|---|
| R1 | Solid rect fill + linear gradient (2-stop) | `crates/renderer/src/fill_paint.rs` |
| R2 | Stroke outline (all edges, uniform width, SDF) | `crates/renderer/src/fill_paint.rs` |
| R3 | Per-corner border radii (independent TL/TR/BL/BR) | `crates/renderer/src/corner_radii.rs` |
| R4 | Drop shadow (box-model outer, Gaussian spread) | `crates/renderer/src/shadow_paint.rs` |
| R5 | Inner shadow (inset blur) | `crates/renderer/src/shadow_paint.rs` |
| R6 | Pattern fill (tiled texture atlas region) | `crates/renderer/src/fill_paint.rs` |
| R7 | Three-way sprite (`Mono` / `Subpixel` / `Poly`) | `crates/renderer/src/scene_command.rs` |
| R8 | Slug icon baker (SVG → quadratic bands → atlas) | `crates/icons/src/` |
| R9 | Text decorations (underline, strikethrough, wavy) | `crates/renderer/src/decoration_paint.rs` |
| R10 | Soft glow (outer bloom via layered radial feather) | `crates/renderer/src/shadow_paint.rs` |
| R11 | Layer pool + two-phase frame walker | `crates/renderer/src/layer_pool.rs`, `layer_walk.rs` |
| R12 | Layer blur (dual-Kawase on layer's own content) | `crates/renderer/src/blur_paint.rs` |
| R13 | Split-path render API (`submit_screen` / `submit_world`) | DR-029 |

### Deferred (no implementation shipped)

| Class | Reason |
|---|---|
| **Radial gradient** (elliptical + circular) | No Pack 3 consumer. Adds a new fragment-shader branch and a new style struct. Defer until a widget (radial-progress, dial) creates a real consumer. |
| **Conic gradient** (CSS `conic-gradient()`) | Extremely rare in desktop UI. Not needed for the Figma-like widget set. Defer to R15+. |
| **Mesh gradient** (four-corner bicubic) | GPU-heavy; no standard rasteriser. Useful for wallpapers, not UI widgets. Defer. |
| **Dotted / dashed stroke** | Requires parameterised dash array, segment walking, and cap shape logic. Only needed for chart/diagram contexts, not the current widget set. Defer. |
| **Double underline / overline decoration** | CSS `text-decoration-style: double` and `overline` were left out of R9. Defer until accessibility or i18n calls for overline in non-Latin scripts. |

### Skipped

| Class | Reason |
|---|---|
| **CSS `box-decoration-break: clone`** | Fragment cloning semantics are orthogonal to the current inline layout model. Sigil's inline layout defers wrap semantics to Parley; per-fragment decoration state would require Parley-level integration that is not planned for R14. |
| **Clip paths (non-rectangular)** | Arbitrary path clips require a stencil pass or GPU conservative rasterisation. The current scene walker only supports axis-aligned `PushClip`. Stencil clips belong in R15+ when the scene graph gains a retained GPU-side representation. |

## Rationale

The shipped set covers every primitive used by Pack 3 widgets (button,
slider, checkbox, text_field, toggle) and by the planned Pack 4 widgets
(panel, progress, dropdown, scroll). Measurement against the widget
implementations shows no widget uses a deferred primitive; every deferred
item has a concrete alternative in the shipped set (a conic progress ring
can be built from a `LayerBlur` + `Mono` mask; a rounded clip can be done
with per-corner radii + transparent rect).

The two skipped primitives would require non-trivial pipeline additions
without a current consumer. Per the verification-honesty principle: no
primitive was added speculatively.

## Consequences

- The thirteen shipped primitives form the stable public surface of
  `SceneCommand`. Any future addition requires a new DR.
- Radial/conic gradients are the most likely next addition. When a widget
  consumer appears, add a single new `fill_paint.rs` fragment branch; no
  new crate is needed.
- Dotted strokes require a new `SceneCommand::DrawStroke` variant (separate
  from the existing `DrawRect` stroke field); plan a DR for that variant
  before implementation.

## Kill Criteria

Revisit if:
- A Pack 4 widget genuinely cannot be built with the current thirteen
  primitives and a deferred primitive is the only correct solution.
- The the host engine consumer screen (R14 Stream E ergonomics check) identifies a
  gap not covered by the shipped set.

## References

- `docs/plans/phase-R1-fill-enum-linear-gradient.md` through `phase-R13-split-path-render-api.md`
- [DR-014](./DR-014-renderer-slug.md) — Slug renderer decision
- [DR-020](./DR-020-scene-command-buffer.md) — Scene command buffer
- [DR-029](./DR-029-split-path-render-api.md) — Split-path render API (R13)
- [DR-032](./DR-032-slug-as-universal-vector-engine.md) — Slug as universal vector engine
- [DR-033](./DR-033-three-way-sprite-split.md) — Three-way sprite split (R7)
- [DR-034](./DR-034-layer-pool-two-phase-frame.md) — Layer pool + two-phase frame (R11)
