# Painting

## Node tree

`sigil::kernel` owns the UI tree. `Cx` is the tree context. `NodeId` is
stable until the node is removed.

`children_reactive` remounts children by index. A length change
rebuilds the list. `children_keyed` remounts by key: a matching key
keeps its `NodeId`. The new builder for a kept key is dropped, so a
dummy `container()` is enough. First key wins on duplicates.

`virtual_list` takes a `Vec` or a `Signal`. Default keys are list
indices. Use `.identified()` when `T: Identifiable`, and `.key_by` for
any other key.

## Layout

`sigil::layout::LayoutEngine` is the only production layout. It mirrors
a kernel subtree, solves flex, block, and grid, and writes
`LayoutResult` back onto the same `NodeId`s. `LayoutStyle` is the only
layout API. `scroll()` is sugar for `Overflow::Scroll`.

`LayoutEngine::compute(cx, root, available)` runs in the Layout phase
when the frame is dirty. A cache key of (signature, available, scale)
skips the solve on a hit. The engine asks a `TextMeasure` for sizes; it
does not shape. Production uses `OwnedTextEngine::thread()`.

## Text

`sigil::text` shapes and measures. HarfRust handles complex shaping,
skrifa loads fonts, and bidirectional text resolves through
`unicode-bidi`. Font fallback selects a face for each run.

## Scene

`emit_scene` walks the laid-out tree and produces a `CommandScene`:
rects, glyph runs, layers, strokes, and shadows. `emit_scene_clipped`
applies a clip. The window viewport clips with 64 px of overscan;
`hidden` and `scroll` ancestors tighten the clip further.

Each node can reuse the previous frame's commands when its content key
matches. The key includes live text, reactive style slices, and
animator overlay values.

## Renderer

`sigil::renderer` is the only module that names wgpu. `LayerPool` is a
free list of offscreen textures. `acquire` reuses or allocates;
`release` returns the texture after submit. A failed acquire is
fail-closed (`LayerError::Empty` or `TooLarge`), never a silent skip.

`run` paints when the frame is dirty or the shell forces a paint. A
forced paint on a clean tree can replay the cached scene.

## Goldens

Visual changes re-bake the goldens. The harness renders a widget
through `WgpuRenderer::render_command_scene_offscreen`, reads the
pixels back, and compares them to a committed PNG under
`crates/sigil/tests/goldens/`. If no GPU adapter is available, the test
skips.
