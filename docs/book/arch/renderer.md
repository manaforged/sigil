# Renderer

`sigil-renderer` is the only crate that names `wgpu`. Everyone else
talks to `sigil_hal::Renderer` or to a `CommandScene`.

## Command scene

`emit_scene` / `emit_scene_clipped` walk the laid-out kernel tree
and produce a `CommandScene`: rects, glyph runs, layers, strokes,
shadows. There is no second `Scene` / `to_flat` adapter.

Clip:

- Window viewport, plus 64 px overscan so a scroll step does not
  pop unshaped rows.
- Overflow `hidden` / `scroll` ancestors tighten the clip.

A `GlyphRun` is `Rc` plus a rect. Replay is cheap.

## Emit cache

Each node can reuse last frame's commands when the content key
matches. The key includes live text, reactive style slices, and
animator overlay values. A motion tick that only changes opacity
misses the cache instead of replaying a frozen rect.

## Layers

`LayerPool` is a free-list of offscreen textures. `acquire` reuses
or allocates. `release` returns the texture after submit. A failed
acquire is fail-closed (`LayerError::Empty` / `TooLarge`), not a
silent skip.

## Paint gate

`launch` paints when `Runtime::frame_dirty` is set or the shell
forces a paint. A forced paint with a clean tree may replay the
cached scene (expose). Animation can force a successor frame.

See [Profiling](../05-profile.md) for `scene.cmds` / `scene.glyphs`
counters.
