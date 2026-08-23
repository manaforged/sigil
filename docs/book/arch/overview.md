# Architecture overview

Sigil is a local desktop UI: a signal graph, a node tree, an owned
layout engine, and a wgpu command scene. `sigil::launch` owns the
window and the frame loop. You write a tree once. Signals change.
The next frame layouts and paints what those writes dirtied.

There is no virtual DOM. There is no per-frame rebuild of the whole
tree. There is no `notify()` after a mutation. A write is
[`Signal::set`](../01-reactive-model.md).

## Data flow

```text
OS event (winit)
    → EventRouter (hit-test last frame's boxes)
    → handler writes a Signal
    → Effect phase runs derived / effect
    → LayoutEngine::compute (if the frame is dirty)
    → emit_scene → CommandScene (skips Surface holes)
    → wgpu present
    → Host::surfaces lists holes (caller places a view)
    → AccessKit tree from the same kernel nodes
```

A game loop skips `launch` and pumps `Host` with its own GPU. TUI
skips wgpu and rasters `painted()` onto cells. The tree is the same.

`launch` also pumps `spawn` completions onto the main thread before
Effect, so a finished future writes its `Signal<Status<…>>` in the
same frame.

## Two trees, one identity

| Tree | Owner | Identity |
| --- | --- | --- |
| Kernel | `sigil-kernel` `Cx` | `NodeId` (stable until the node is removed) |
| Layout mirror | `sigil-layout` `LayoutEngine` | `NodeHandle` mapped from `NodeId` |

The kernel tree is the source of truth. Layout mirrors it, computes
boxes, and writes `LayoutResult` back onto the same nodes. The
renderer and the AccessKit bridge read those nodes. They do not own
a second widget graph.

## What you write

```rust,ignore
use sigil::prelude::*;

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().gap(8.0))
            .child(text(derived(move || count.get().to_string())))
            .child(button("+").activate(move || count.set(count.peek() + 1))),
    )
}
```

- `Cx` is the tree context. You add nodes. You do not implement
  `Render` on a type.
- `Signal` is app state. You do not construct an `Entity`.
- `LayoutStyle` is the only layout API. Widgets are sugar.

## What this is not

GPUI's unit of UI is an `Entity` that owns state, implements `Render`,
and calls `cx.notify()` after a mutation. That model is documented
in [Signals, not entities](signals.md). Sigil keeps an owner tree,
a focus stack, action dispatch, and phased frames — those jobs are
real. It does not make `Entity` the author type for app state.

## Where the rest lives

| Topic | Chapter |
| --- | --- |
| Why `Signal` | [Signals, not entities](signals.md) |
| Dispose and cleanup | [Owner tree](owners.md) |
| Phase order | [Frame loop](frame.md) |
| `NodeId`, keyed children | [Node tree](tree.md) |
| Incremental boxes | [Layout engine](layout.md) |
| Command scene | [Renderer](renderer.md) |
| `spawn` | [Async](async.md) |
| AccessKit | [Accessibility](a11y.md) |
| Crate owners | [Crate map](crates.md) |
