# DR-017 — Default scene paint in `sigil-renderer`

**Status:** Accepted, text-deferral clause superseded by [DR-025](DR-025-text-in-scene-interim.md) — rect-pipeline design retained.
**Date:** 2026-04-16
**Depends on:** DR-001 (renderer wgpu), DR-014 (slug text)
**Unlocks:** DR-016 (app launch), Phase 11 (scene command buffer)

## Context

Before this DR, `WgpuRenderer` exposed `render_frame(surface, clear, draw)`
and left rect painting to each consumer. Every example reimplemented the
same wgpu setup: a `RectVertex` struct, a WGSL shader string, a render
pipeline built per frame (or, at best, cached at startup), a vertex
buffer keyed by scene size.

That 150 lines of boilerplate is the second-largest duplication in the
repo after launch orchestration (DR-016), and it has two concrete
costs:

1. Every new consumer of `Scene` (widgets crate, test fixtures, future
   examples) reimplements the same pipeline with subtle differences.
2. Adding rounded corners or opacity becomes a per-consumer decision
   rather than one shared implementation.

The phase-08a plan called for a `WgpuRenderer::render_scene` method
that the renderer owns. This DR records that addition and the design
of the internal pipeline.

## Decision

`WgpuRenderer` gains a public method:

```rust
pub fn render_scene(
    &mut self,
    surface: &mut WgpuSurface,
    scene: &Scene,
    clear_color: Color,
    viewport: (f32, f32),
) -> Result<(), RendererError>;
```

It acquires the surface texture, clears, paints every `SceneItem::Rect`
using an internal instanced pipeline, and presents. `SceneItem::Text`
is logged once per process (warn) and dropped — text paint lands in
Phase 14 (text-in-scene + IME).

### Internal pipeline design

- **Instanced rendering.** A fixed 6-vertex unit quad `[0,0]..[1,1]` +
  per-instance `(rect_xywh, color_rgba, radius_opacity)`. One draw call
  per scene, regardless of rect count.
- **Viewport uniform.** A single `vec2<f32>` uniform buffer (logical
  pixels) used by the vertex shader to convert instance rects into NDC.
  No CPU-side NDC conversion.
- **Rounded corners via SDF.** The fragment shader computes a rounded-
  rectangle signed distance field when `radius > 0` and anti-aliases
  via `smoothstep(-0.5, 0.5, d)`. `radius == 0` skips the SDF entirely.
- **Pre-multiplied alpha via `BlendState::ALPHA_BLENDING`.** Opacity
  multiplies into the alpha channel per fragment.
- **Format-keyed pipeline cache.** `WgpuRenderer` stores a
  `Vec<ScenePipeline>` in `ReadyState`; lookup is linear scan by
  `wgpu::TextureFormat` (expected size: 1–2 entries). Multiple surfaces
  with the same format share a pipeline.
- **Growing instance buffer.** Buffer capacity doubles on overflow; the
  initial capacity is 64 instances. `queue.write_buffer` re-uploads
  every frame (zero-copy on current wgpu, CPU → staging → GPU in older
  backends).

### Text (explicitly deferred)

`render_scene` does **not** paint text. The Slug pipeline
(`crates/sigil-renderer/src/slug/`) remains callable via `render_frame`
for consumers that need text today. Wiring text into `render_scene` is
Phase 14 and blocks on the scene command-buffer rework (DR-020, post-
Spike 3).

### Crate layout

Renderer internals split across three modules:

- `renderer.rs` — public API: `WgpuRenderer::new` / `render_frame` /
  `render_scene` plus `Renderer` trait impl.
- `device_state.rs` — `DeviceState` / `ReadyState` with lazy
  instance → adapter+device+queue+pipelines promotion on first
  `create_surface`. Split out to respect the 400-line hygiene cap.
- `scene_paint.rs` — `ScenePipeline` with the instanced rect pipeline,
  shader, and vertex/instance layouts.

## Rationale

**Why instanced, not one triangle strip per rect.** At 10 000 rects the
instanced path uploads one buffer and issues one draw call. A
non-instanced path would upload 60 000 vertices per frame and spend
noticeably more CPU time in the wgpu encoder. DR-009's data-heavy
performance posture makes instanced the obvious default.

**Why SDF corners in the fragment shader, not tessellation.** SDF gives
pixel-perfect anti-aliasing at any radius without tessellation cost.
The alternative — subdividing rect corners into fans of triangles — has
worse aliasing and higher vertex count. SDF is what every modern 2026
renderer (Vello, Skia, Compose) ships.

**Why a format-keyed cache, not a single pipeline.** Surfaces with
different formats can coexist (a secondary debug overlay window could
be sRGB while the main is HDR). Linear-scan-with-Vec is fine at
1–2 formats; upgrade to HashMap if we ever exceed 4.

**Why grow the instance buffer instead of reallocating per frame.** A
grow-doubling buffer amortizes allocation to O(log N) in the lifetime
of the app and hits zero allocations on the steady-state case (same
tree every frame → same size).

**Why defer text.** Text paint through a flat `Scene` is wrong — the
shader needs per-glyph atlas coordinates that the current `SceneItem::Text`
doesn't carry. The correct fix is a command-buffer scene with
`DrawText { glyph_run }` commands (DR-020 / Phase 11). Shipping a
half-baked text path today would be thrown away in a month.

## Consequences

- `render_scene` is the default for app authors through
  `sigil::launch`. `render_frame` remains for advanced consumers that
  want to composite custom passes (e.g. hello-rect's pre-DR form,
  Slug text today).
- Consumers get rounded corners and opacity for free — no per-app
  shader work.
- The 150 lines of rect-pipeline boilerplate that hello-style and
  hello-events carried vanish from the ported examples.
- A future `DrawImage` / `DrawText` extension lands as additional
  instance-buffer formats in `scene_paint.rs` or as separate
  sub-pipelines behind the same public method. No downstream consumer
  breaks.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Ship rect painting in a separate crate | Fragments the "use WgpuRenderer" story; forces every app to pick up a second crate. |
| CPU-side rect buffer with `render_frame` callback | Pushes the 150-line shader dance back into user code — exactly the problem this DR solves. |
| Vello-style compute pipeline | Disproportionate dependency cost for filled rects. DR-014 already identified Vello as out of scope. |
| Text paint inside `render_scene` today | Scene model needs command-buffer rework first (Phase 11). Half-baked text goes in the trash. |

## References

- [`docs/plans/phase-08a-ergonomics-pass.md`](../plans/phase-08a-ergonomics-pass.md)
- [`crates/sigil-renderer/src/renderer.rs`](../../crates/sigil-renderer/src/renderer.rs)
- [`crates/sigil-renderer/src/scene_paint.rs`](../../crates/sigil-renderer/src/scene_paint.rs)
- [`crates/sigil-renderer/src/device_state.rs`](../../crates/sigil-renderer/src/device_state.rs)
- [`crates/sigil-renderer/src/scene.rs`](../../crates/sigil-renderer/src/scene.rs)

## Kill Criteria

Revisit if:

- Phase 11 scene command buffer (DR-020, pending Spike 3) replaces
  `SceneItem::Rect` wholesale — at which point `ScenePipeline`'s input
  format changes and this DR supersedes itself.
- Benchmarks show the grow-doubling instance buffer costs more than
  a ring buffer at 10k+ rects per frame (unlikely — one memcpy at most
  per frame resize).
- Upstream wgpu introduces a native rounded-rect primitive (no sign of
  this as of 2026-04).
