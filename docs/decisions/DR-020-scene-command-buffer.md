# DR-020 - Scene command buffer

**Status:** Accepted (ratified as Phase 11 Step 1, 2026-04-18).
**Date:** 2026-04-18
**Depends on:** DR-017 (default scene paint), DR-025 (text-in-scene interim), Spike 3 (`docs/spikes/03-scene-model.md`)
**Unlocks:** Phase 11 (scene command buffer), Phase 12 (scrollview), Phase 13 (overlay/portal), Phase 14 (proper text-in-scene + IME)

## Context

The current renderer scene model is a flat `SceneItem` list with
`Rect` and `Text` variants. That shape was enough for DR-017's default
rect paint path and DR-025's interim text-in-scene wiring, but the next
roadmap phases need scoped paint state:

- scrollview clips and scissors,
- transforms for nested coordinate spaces,
- opacity/saveLayer groups for popovers and dialogs,
- glyph-run commands for the post-interim text path.

Spike 3 compared three CPU-side scene models:

- **A - Flat enum:** every draw item carries absolute clip, transform,
  and layer state.
- **B - Command buffer:** explicit push/pop state commands plus draw
  commands.
- **C - Hybrid:** keep flat public items and compile them to commands
  in the renderer.

## Decision

Adopt **Variant B - command buffer** for Phase 11, with an ergonomic
emitter facade so widget code does not manually manage raw push/pop
commands.

The command vocabulary should start small:

```rust
enum SceneCommand {
    PushClip(Rect),
    PopClip,
    PushTransform(Transform),
    PopTransform,
    DrawRect(...),
    DrawGlyphRun(...),
    SaveLayer { opacity: f32 },
    RestoreLayer,
}
```

The public authoring surface should be a scene/emitter builder that
guarantees balanced scopes. Raw opcodes can remain renderer-internal or
low-level until real consumers prove they need direct access.

## Rationale

Spike 3 measured build, walk, and dispatch-prep cost for flat, clipped,
and layered scenes. Variant B was fastest or tied in the meaningful rows:

| Scenario | A median | B median | C median |
| --- | ---: | ---: | ---: |
| Sc1 flat-1k | 4.8957 us | **3.9987 us** | 5.7773 us |
| Sc2 clipped-100 | 9.1091 us | **9.1306 us** | 13.475 us |
| Sc3 layered-10 | 2.5261 us | **2.2439 us** | 3.7536 us |

Sc2 is a practical tie between A and B, but B has the stronger semantic
model. It represents clips, transforms, and layers as scopes instead of
duplicating absolute state onto every draw item.

B also has the best fit for the roadmap:

- Phase 12 scrollview maps naturally to `PushClip` / `PopClip`.
- Phase 13 overlay and portal work needs explicit layer and transform
  boundaries.
- Phase 14 can replace `SceneItem::Text` with `DrawGlyphRun` without
  baking glyph atlas details into widget code.

Variant C preserves flat emission but pays an extra compile step and
allocates the most in every measured scenario. It is useful as a fallback
if public compatibility becomes the overriding constraint, but it should
not be the default Phase 11 design.

## Consequences

- Phase 11 will replace or supersede the current flat `SceneItem` walk
  with a command-buffer walk.
- Renderer internals can prepare draw packets from explicit state stacks
  instead of reconstructing state from repeated draw metadata.
- Widget authors need an ergonomic emitter facade so ordinary widgets
  still write simple code.
- Existing examples and `sigil::launch` should not expose raw command
  buffers unless an advanced custom renderer path explicitly asks for it.
- DR-025's interim text path remains valid until Phase 11 lands; then
  glyph shaping moves toward command emission and `DrawGlyphRun`.

## Alternatives Rejected

| Alternative | Decision | Reason |
| --- | --- | --- |
| A - flat enum | Parked | Easy to emit, but nested clip/transform/layer semantics become repeated per-item metadata and are harder to audit for overlays/portals. |
| C - hybrid compiler | Parked | Preserves flat emission but has the highest allocation profile and relies on reconstructing scopes from flat metadata. |

## Kill Criteria

Revisit this DR if:

- The Phase 11 emitter facade cannot make scoped commands safe for widget
  authors without exposing raw push/pop management.
- A real scene profile shows command interpretation dominating frame time
  after renderer batching is implemented.
- Overlay/portal implementation needs cross-tree composition semantics
  that do not fit stack-scoped commands.
- AccessKit or IME work requires text commands to carry state that is
  better represented as a retained scene graph than a command buffer.
- A future renderer backend requires a radically different ordering model
  for saveLayer or glyph atlas dispatch.

## References

- `docs/spikes/03-scene-model.md`
- `docs/spikes/03-scene-model-brief.md`
- `crates/sigil/renderer/benches/scene_model_rig.rs`
- `docs/decisions/DR-017-scene-paint.md`
- `docs/decisions/DR-025-text-in-scene-interim.md`
