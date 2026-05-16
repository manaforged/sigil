# DR-029 — Split-path render API for engine integration

**Status:** Draft
**Date:** 2026-05-14
**Depends on:** DR-001 (renderer wgpu), DR-017 (default scene paint), DR-020 (scene command buffer)
**Unlocks:** the host engine engine integration (`crates/the host engine/ui` consuming Sigil mid-frame), any future game engine adopting Sigil, world-space text in `the host engine/ui-world`

## Context

`WgpuRenderer::render_scene` (DR-017) and the scene-command interpreter
(DR-020) currently assume Sigil owns the whole frame: clear → paint
scene → present, all behind one call. That matches `sigil-app`'s
desktop-app use case where Sigil is the only renderer on the window.

It does not match an engine consumer. the host engine has three other renderers
(sprite, character, mesh) that already cooperate inside a single
`WindowedRunner` frame:

```
mesh (opaque + shadow) → sprite → character → lights → post
```

the host engine needs Sigil to slot into that frame *twice*:

- **World-space text** (`the host engine/ui-world`, 3D nameplates, damage
  numbers, in-world signs) renders into the 3D depth buffer between
  the character pass and the lights pass.
- **Screen-space UI** (`the host engine/ui`, HUD/menus/dialogs) renders as the
  final pass over the swapchain, alpha-over the post-processed scene.

Both consume the *same* scene command buffer, glyph cache, and Slug
band textures. Only the GPU lifecycle — who clears, who acquires the
surface, who presents — differs.

Sigil today cannot do this. `render_scene` acquires the surface,
clears it, and presents on return; an engine cannot insert other
passes in between, and cannot share its own `CommandEncoder` with
Sigil for command-buffer batching.

## Decision

Refactor `sigil-renderer` to expose **two narrow submission entry
points** alongside the existing `render_scene`. The new entry points
do not touch surface acquisition or presentation — those stay with
the caller.

### New API

```rust
pub struct ScreenSubmission<'a> {
    pub view: &'a wgpu::TextureView,
    pub viewport: PhysicalSize,
    pub dpi: f32,
    pub load: wgpu::LoadOp<wgpu::Color>,  // Clear(...) | Load
}

pub struct WorldSubmission<'a> {
    pub color_view: &'a wgpu::TextureView,
    pub depth_view: &'a wgpu::TextureView,
    pub view_proj: [[f32; 4]; 4],
    pub viewport: PhysicalSize,
}

impl WgpuRenderer {
    /// Encode Sigil's screen-space scene into `encoder`. Orthographic
    /// projection, no depth test, alpha-over premultiplied. The caller
    /// owns the encoder and any swapchain present that follows.
    pub fn submit_screen(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneCommands,
        target: ScreenSubmission<'_>,
    ) -> Result<(), RendererError>;

    /// Encode Sigil's world-space scene into `encoder`. Billboarded
    /// glyph quads at world anchors, sampled against `depth_view` for
    /// occlusion against the host's 3D scene. Caller owns the encoder.
    pub fn submit_world(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneCommands,
        target: WorldSubmission<'_>,
    ) -> Result<(), RendererError>;
}
```

Both methods share the existing pipeline cache, glyph cache, and Slug
band textures. `submit_screen` reuses the rect+text pipelines DR-017
introduced; `submit_world` adds a depth-aware variant that samples
`depth_view` per-fragment.

### `render_scene` becomes a thin wrapper

```rust
impl WgpuRenderer {
    pub fn render_scene(
        &mut self,
        surface: &mut WgpuSurface,
        scene: &SceneCommands,
        clear_color: Color,
        viewport: PhysicalSize,
    ) -> Result<(), RendererError> {
        let frame = surface.acquire()?;
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device().create_command_encoder(&Default::default());
        self.submit_screen(
            &mut encoder,
            scene,
            ScreenSubmission {
                view: &view,
                viewport,
                dpi: surface.dpi(),
                load: wgpu::LoadOp::Clear(clear_color.into()),
            },
        )?;
        self.queue().submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}
```

`sigil-app` keeps calling `render_scene`. Behaviour for existing
consumers is byte-identical.

### Why two entry points, not one

A unified `submit(encoder, scene, target)` with a target enum mixes
two genuinely different pipelines: screen-space is no-depth ortho,
world-space is depth-sampled with a view-projection matrix. A target
enum forces every call site through a match and pushes optional
fields (`depth_view: Option<...>`, `view_proj: Option<...>`) into the
non-applicable case. Two methods with two narrow structs read
honestly at the call site.

### Lifecycle ownership table

| Concern | `render_scene` | `submit_screen` | `submit_world` |
|---|---|---|---|
| Surface acquire | Sigil | **Caller** | **Caller** |
| Clear / load op | Sigil (always clears) | **Caller chooses** | **Caller** (typically Load) |
| Command encoder | Sigil creates | **Caller provides** | **Caller provides** |
| Queue submit | Sigil | **Caller** | **Caller** |
| Surface present | Sigil | **Caller** | **Caller** |
| Glyph cache, pipelines, band texture | Sigil | Sigil (shared) | Sigil (shared) |
| Depth-buffer interaction | None | None | Reads `depth_view` |

## Consequences

- the host engine can wire Sigil into [`platform::WindowedRunner`](../../the host engine/crates/platform/src/window.rs)'s
  render loop without forking. `submit_world` slots between the
  character and lights passes; `submit_screen` runs as the final pass
  after post-processing.
- Any future game engine adopting Sigil gets the same integration
  story for free — the API is engine-shape, not Sigil-shape.
- `sigil-app` carries no behaviour change. Existing examples
  (`hello-rect`, `hello-style`, `hello-events`, vertical-slice) keep
  using `render_scene` and produce byte-identical frames.
- Glyph cache and Slug band textures live on `WgpuRenderer` and are
  shared across both entry points; no per-pass duplication.
- Surface lifecycle complexity moves to consumers. Documented in the
  new API rustdoc; sample integration in
  `examples/embedded-renderer/` (added with the implementation).
- Pipeline cache grows by one *kind* — a depth-aware variant of the
  text pipeline — multiplied across the existing cache key axes
  (surface format, msaa, future colour-space). Rect/non-text
  pipelines are unchanged. Concretely: each surface format in use
  gains one extra cached pipeline.
- World-space text adds a new shader uniform (`view_proj`) and a
  per-instance world-anchor attribute. Documented in
  `crates/sigil/renderer/src/world_text.rs` (added with the
  implementation).

## Kill criteria

Revisit this DR if any of these turn out true:

- The world-space rendering need disappears — every adopter ends up
  only using `submit_screen`. In that case collapse to a single
  entry point and drop `submit_world`.
- A future render-graph crate (Sigil owning its own scheduler,
  bevy-style) makes split-path submission unnecessary because the
  scheduler composes passes from the renderer's side. Re-evaluate
  whether the narrow entry points are still the right shape.
- the host engine moves off the wgpu primary path (e.g. adopts a SDL-based
  shell). The depth-view contract assumes wgpu textures; a different
  backend would force a HAL-level abstraction here.
- The pipeline-cache shared-mutable-state invariants prove
  impractical to enforce across two entry points called from
  different threads. (Sigil is main-thread-only today, so this is a
  future-look concern.)
