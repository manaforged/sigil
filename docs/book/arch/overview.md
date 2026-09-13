# Architecture overview

One frame has three phases: Effect, Layout, and Paint. `sigil::launch`
owns the window and the frame loop.

Signals are written in Effect. Event handlers, animation ticks, and
task completions write there. A write with live subscribers marks the
frame dirty. A signal write during Paint panics in a debug build and
can paint a stale frame in a release build. Write signals in Effect,
not in Paint. Layout reads signals for reactive style and children.
Paint reads the laid-out tree.

Data moves in this order:

```text
OS event (winit)
    → EventRouter (hit-test on last frame's boxes)
    → handler writes a signal
    → Effect: derived values and effects run
    → Layout: LayoutEngine::compute when the frame is dirty
    → Paint: emit_scene builds a CommandScene
    → wgpu presents
    → Host::surfaces lists holes for native views
    → AccessKit projects the same kernel nodes
```

A clean frame skips layout and scene assembly. An idle window does not
spin the loop. The waker from input, a task, a menu, or a tray event
starts the next frame.

A game loop skips `launch` and pumps `Host` with its own GPU. The
terminal host skips wgpu and rasters the painted nodes onto cells.

For module ownership, see [Modules](modules.md).
