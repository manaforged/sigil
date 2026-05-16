# DR-025 — Text-in-scene interim wiring (pragmatic Phase 14 stub)

**Status:** Superseded by DR-020 (Phase 11 realised the long-term design; the interim path is retired)
**Date:** 2026-04-17
**Supersedes in part:** DR-017 (retains the `render_scene` rect design; replaces the "text is dropped with a one-per-process warn" clause).
**Depends on:** DR-001 (wgpu renderer), DR-004 (Parley text engine), DR-014 (Slug glyph pipeline), DR-017 (scene paint).
**Does not replace:** DR-020 / Phase 11 scene command buffer — the command-buffer rework still lands later and will re-home this code.

## Context

DR-017 shipped `WgpuRenderer::render_scene` as the default paint path, with `SceneItem::Text` explicitly deferred:

> `render_scene` does **not** paint text. [...] Wiring text into `render_scene` is Phase 14 and blocks on the scene command-buffer rework (DR-020, post-Spike 3).

The rationale in DR-017 was that a flat `Scene` can't carry per-glyph atlas coordinates without a command-buffer rework. That remains true for the *correct long-term* design.

The vertical-slice example (`examples/vertical-slice`) needs visible text to be a slice at all — a dashboard with invisible labels is not a proof-of-concept. The cost of waiting for Phase 11 (scene command buffer) + Phase 14 (text-in-scene + IME) before demoing an interactive app was unacceptable given that Phases 11–13 are still unshipped.

## Decision

Wire `SceneItem::Text` through the existing Slug glyph pipeline directly from `render_scene`, as an interim path that will be replaced by Phase 11's command-buffer rework.

### Shipped design

1. `ReadyState` gains two fields alongside `scene_pipelines`:
   - `slug_pipelines: Vec<SlugTextPipeline>` — format-keyed cache mirroring the rect path.
   - `text_engine: ParleyTextEngine` — one engine per renderer for glyph shaping.
2. `render_scene` runs two passes: rect pass (unchanged from DR-017), then a text pass that walks `SceneItem::Text` entries and draws each paragraph via `SlugTextPipeline::draw_paragraph`.
3. The text pass lives in a separate module (`crates/sigil/renderer/src/text_paint.rs`) to keep `renderer.rs` under the 400-line hygiene cap.
4. Paragraph cache (`ParleyTextEngine::cache`) persists across frames — identical `(text, max_width)` pairs hit on repaint.

### Why the cache is not cleared per-frame

An earlier version of this code cleared the paragraph cache at the top of every text pass. That re-shaped every paragraph every frame, defeating the cache's entire purpose. This DR codifies: **the renderer does not own cache lifecycle**; callers that need a clear cycle call `TextEngine::clear_cache()` themselves.

## Rationale

**Why ship interim instead of waiting.** Phase 11 is a spec of uncertain size (command buffer + `DrawText { glyph_run }` commands + encoder reconciliation). The vertical slice is a shipping commitment. Waiting for Phase 11 to demo an interactive app would have pushed the vertical slice past a reasonable horizon.

**Why this is acceptable despite DR-017 saying "half-baked text goes in the trash."** DR-017's concern was per-consumer shader dances and atlas coords baked into `SceneItem::Text`. This interim path doesn't put atlas coords in `SceneItem::Text` — the `SceneText` struct stays clean (`text`, `rect`, `color`, `opacity`). Glyph shaping happens inside the renderer, not in scene emission. When Phase 11 lands, it will replace the call from `render_scene` to `paint_text_items` with a command-buffer walk, and `SceneItem::Text` will be retired or reshaped. The scene emitter and the widget layer don't change.

**Why one pipeline per format, not one per scene.** Same rationale as DR-017 for `scene_pipelines`: a renderer can drive multiple surfaces; formats can coexist. Linear scan is fine at 1–2 formats.

**Why one `ParleyTextEngine` per renderer, not per surface.** Glyph caches are hot; font context initialization is 10–50 ms (DR-004). Sharing across surfaces is strictly better.

## Consequences

### What changes from DR-017

- **Text is no longer dropped from `render_scene`.** The "one-per-process warn and drop" code (`warn_unsupported_text_once`) is removed.
- **`ReadyState` grows two fields.** Not a public API change; `ReadyState` is `pub(crate)`.
- **`renderer.rs` splits.** Text-paint code lives in `crates/sigil/renderer/src/text_paint.rs`; `renderer.rs` imports from it.
- **`DrawParams::surface_width`/`surface_height` are now `f32`.** DR-017's design used `u32` implicitly (physical pixels); the viewport this path uses is logical pixels per DR-018, and fractional DPI needs `f32`. Internal callers updated; no public API is on this path yet.
- **`render_style_for` defaults text nodes to `on_surface` theme color.** Without this, un-styled text rendered as `Color::WHITE` from the glyph pipeline's builder default.

### What stays from DR-017

- `render_scene(surface, scene, clear_color, viewport)` signature — unchanged.
- Rect pipeline design (instanced, SDF corners, format-keyed cache) — unchanged.
- The pre-existing `render_frame` fallback for consumers that want custom composites — unchanged.

### Phase 11 compatibility

When Phase 11's scene command buffer lands, the expected migration is:

1. `SceneItem::Text` is replaced by a `DrawText { glyph_run }` command carrying already-shaped glyphs.
2. The glyph-shaping step currently inside `paint_text_items` moves to whoever emits the command buffer (widget layer or scene encoder).
3. `text_paint.rs`'s outer loop walks commands instead of scene items; `SlugTextPipeline::draw_paragraph` becomes `SlugTextPipeline::draw_glyph_run`.
4. `ParleyTextEngine` stays in `ReadyState` but only handles glyph atlas management, not shaping.

No public API on the `sigil` crate surface depends on the current `SceneItem::Text` shape, so this migration has no downstream-app breakage risk.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Wait for Phase 11 before demoing interactive slice | Pushes vertical slice past its useful horizon; Phases 11–13 are unsized. |
| Emit shaped glyphs from the widget layer directly into `SceneItem::Text` | Bakes atlas coords into the scene — exactly what DR-017 rejected. |
| Put glyph shaping on a background thread | Phase 09 reactive recomposition concern; out of scope here. |
| Keep the paragraph cache per-frame-cleared | Re-shapes every paragraph on every frame; defeats Parley's performance model. |

## Kill criteria

Retire this DR when:

- Phase 11 scene command buffer ships and `SceneItem::Text` is superseded by `DrawText { glyph_run }`. At that point this DR's implementation is deleted and the migration notes above execute.

Revisit (do not retire) if:

- A real app's profile shows per-frame paragraph shaping dominating even with cache hits — in which case the cache insertion strategy changes, but the overall two-pass architecture stays.
- IME integration (Phase 14 proper) requires text-painting decisions this DR didn't anticipate — then add a follow-up DR for IME without replacing this one.

## References

- DR-017 — default scene paint (superseded in part by this DR).
- DR-004 — Parley text engine (source of `ParleyTextEngine`).
- DR-014 — Slug glyph renderer (source of `SlugTextPipeline`).
- DR-018 — DPI unit system (rationale for `f32` viewport).
- `crates/sigil/renderer/src/text_paint.rs` — the shipped implementation.
- Commit `9436dfa` — "feat(vertical-slice): interactive dashboard proof + text-in-scene wiring".
