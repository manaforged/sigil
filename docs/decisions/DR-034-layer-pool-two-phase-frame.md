# DR-034 — Layer Pool + Two-Phase Frame Walker

**Status:** Accepted
**Date:** 2026-05-15
**Covers:** Phase R11 (layer pool + frame walker) and R12 (LayerBlur)
**Depends on:** DR-020 (scene command buffer)
**Unlocks:** R12 (LayerBlur), R14.A3 (LayerBlur rename), R15 (true backdrop snapshot)

## Context

Dual-Kawase blur (R12) and any future post-process effect that reads from the
render target before writing back to it face a fundamental GPU constraint: a
fragment shader cannot read the framebuffer texture while writing to it in the
same render pass. The framebuffer is write-only within a pass.

The naive fix — read-back via a copy — requires submitting and completing the
GPU encoder before the compositor can sample the result, stalling the CPU and
breaking frame pipelining. The production solution (used by KDE KWin, Figma,
niri) is to redirect the affected content to an offscreen texture *before*
reading it, then composite the processed texture back into the main frame.

Two sub-problems needed separate solutions:

1. **Offscreen texture lifecycle.** Allocating a new `wgpu::Texture` per
   blurred layer per frame causes GPU memory fragmentation and rebuild of
   the bind group on every frame. Textures need to be pooled and reused.

2. **Frame traversal order.** The existing single-pass command-buffer walker
   (DR-020) processes commands front-to-back in emission order. It cannot
   express "render all children of this layer into an offscreen texture, then
   apply an effect, then composite back at this z-position." That requires
   a tree-shaped traversal rather than a flat walk.

## Decision

Ship two cooperating pieces of infrastructure:

### 1. LayerPool — 4-entry LRU dimension table

```rust
pub struct LayerPool {
    slots: Vec<Slot>,   // ≤ MAX_ENTRIES = 4
    frame: u64,
}
```

The pool does not cache `wgpu::Texture` objects between frames — each
`acquire()` always allocates a fresh texture. What it caches is the set of
recently-used `(width, height, format)` dimension keys (up to 4 entries),
tracked by `last_used_frame`. When the table is full and a new distinct
dimension is requested, the oldest-used slot is evicted.

Dimension normalisation: both axes are rounded up to the nearest 16-pixel
multiple before the lookup. This raises the probability that successive frames
with slightly-shifted clip rects share a pool key rather than thrashing the
LRU table. Transparent edge bytes outside the logical clip are harmless
during composition.

Each handle owns its `wgpu::Texture` and releases it on drop. There is no
`release()` path because `wgpu` does not support texture reuse across frames
without an explicit sync point; a fresh allocation per frame at the already-
paged-in dimension is cheaper than explicit inter-frame sharing.

**Why 4 entries?** A typical frame with blur has at most 2–3 simultaneously
live offscreen layers (modal overlay + a nested blur card). 4 entries covers
this with one spare. Growing the table beyond 8 entries starts to matter for
memory; shrinking below 2 causes thrashing on the common modal case.

### 2. Build-tree + render-nodes two-phase walker

The layer walker in `crates/renderer/src/scene_command/layer_walk.rs` replaces
the flat `Walk` with a two-phase traversal:

**Phase 1 — build_tree:** scan the flat `&[SceneCommand]` slice and construct
an in-memory `Vec<Node>` tree. `PushLayer` / `PopLayer` pairs become
`Node::Layer { scope, children }` subtrees; all other commands become
`Node::Command(SceneCommand)` leaves.

```rust
enum Node {
    Command(SceneCommand),
    Layer { scope: LayerScope, children: Vec<Node> },
}
```

**Phase 2 — render_nodes:** walk the tree depth-first. For each `Node::Layer`,
call `render_layer`:
1. Acquire an offscreen `LayerHandle` from the `LayerPool`.
2. Clear the offscreen texture.
3. Recurse into the layer's children, rendering them into the offscreen view.
4. Apply `LayerScope::effect` (currently `Identity` or `LayerBlur`).
5. Composite the offscreen view back into the parent view at `LayerScope::clip`
   with `LayerScope::opacity`.

Sibling `Node::Command` sequences are batched into a single `Walk` so that
`PushClip` / `PushTransform` / `SaveLayer` state mutations persist across
consecutive commands within the same layer scope.

### LayerEffect on the public API

```rust
pub enum LayerEffect {
    /// No effect — composite the layer's content as-is.
    Identity,
    /// Dual-Kawase blur applied to the layer's own rendered content.
    LayerBlur { sigma: f32 },
}
```

`Identity` is R11's shipping variant (opacity layers). `LayerBlur` is R12's
addition. The variant name was originally `BackdropBlur` (following CSS
`backdrop-filter` naming) but was renamed to `LayerBlur` in R14.A3 because
it blurs the layer's own content, not the parent backdrop. The CSS name is
reserved for a future R15 variant that captures the parent buffer before the
layer renders and blurs that snapshot.

### PushLayer / PopLayer scene commands

```rust
pub enum SceneCommand {
    // existing variants...
    PushLayer(LayerScope),
    PopLayer,
}
```

`PushLayer` / `PopLayer` are balanced by the `SceneBuilder` facade; raw
emission is an internal implementation detail. Widget authors use
`SceneBuilder::push_layer(scope)` and the compiler ensures `pop_layer` is
called via a guard type (future R15 work; currently the caller must balance
manually).

## Consequences

- Any `PushLayer` / `PopLayer` pair forces an offscreen render-to-texture
  pass. Use only for effects that genuinely require it (blur, opacity < 1.0,
  future backdrop snapshot). A plain `PushClip` / `PopClip` is free.
- The LayerPool lives on `WalkCtx` and is lazily initialised (`get_or_insert_with`).
  Frames without any `PushLayer` commands pay zero allocation cost.
- The build-tree phase allocates a `Vec<Node>` per frame for any scene with
  layers. For the common case (no layers, flat scene), `build_tree` returns a
  flat `Vec<Node::Command>` and render_nodes degrades to the same single-Walk
  path as the previous implementation.
- Nesting limit: the walker asserts `layer_depth < 8` in debug builds. A layer
  nesting depth of 8 is far beyond any realistic UI scene; the assert catches
  accidental unbounded recursion in malformed scenes.
- R15 backdrop snapshot (parent buffer capture + blur-under composite) requires
  no new infrastructure — it adds a `BackdropSnapshot` variant to `LayerEffect`
  and a capture step in `apply_effect_and_composite`. The pool and walker do not
  change.

## Kill Criteria

Revisit if:
- Frame profiling shows `build_tree` allocation is measurable (> 0.1 ms) in a
  scene with > 100 layers. In that case, switch to an arena allocator or a
  persistent tree rebuilt only on scene change.
- The 4-entry pool proves insufficient in practice (observable GPU memory
  allocation spikes per frame with > 4 simultaneously visible blur layers).
  Raise `MAX_ENTRIES` to 8 and add a frame-budget test.
- A render-graph abstraction is added to `sigil-renderer` that can express
  inter-pass dependencies declaratively. If so, `LayerPool` becomes a resource
  allocated by the render graph, and the two-phase walker is replaced by the
  graph scheduler.

## References

- `docs/plans/phase-R11-offscreen-layer-pool.md`
- `docs/plans/phase-R12-backdrop-blur.md`
- `crates/renderer/src/layer_pool.rs` — LayerPool implementation
- `crates/renderer/src/scene_command/layer_walk.rs` — two-phase walker
- `crates/renderer/src/layer_composite.rs` — LayerCompositePipeline
- `crates/renderer/src/blur_paint.rs` — dual-Kawase pipeline (R12)
- [DR-020](./DR-020-scene-command-buffer.md) — Scene command buffer (PushLayer / PopLayer origin)
- [DR-031](./DR-031-vector-primitive-set.md) — Vector primitive set (R11/R12 in shipped list)
