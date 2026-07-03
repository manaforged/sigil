---
id: DR-054
status: accepted
date: 2026-05-20
covers: Host-agnostic UI context — one sigil core, two hosts (desktop shell + embedded game engine)
relates: [DR-034 (two-phase frame walker — borrowed shape), engine_adapter crate]
supersedes_direction: engine_adapter "sigil-drives-the-engine" contract
---

# DR-054 — Host-agnostic UI context (desktop shell + embedded the host engine)

**Status:** Accepted · **Date:** 2026-05-20

## Context

Sigil must serve two consumers from one codebase:

1. **Desktop apps** — sigil owns the OS window, event loop, swapchain, and
   frame timing. Entry point `sigil::launch(config, tree)`.
2. **Embedded in the host engine** (the game engine) — *the host engine* owns the window, wgpu
   device/queue, and the fixed-tick + redraw loop (the
   `the host engine::window(...).run(...)` bridge). Sigil renders UI (text/HUD/menus)
   as a layer composited into the host engine's frame.

The **only** thing that differs between these is *who owns the
window/loop/frame*. Layout, reactivity, the widget tree, text shaping
(`text`), scene emission (`emit_scene`), and the GPU rasteriser (`renderer`,
Slug) are identical for both.

The `engine_adapter` crate was a first attempt at this seam but is shaped two
ways wrong for the embedded case: (a) it makes **sigil drive** the frame
(sigil issues `Clear`/`DrawScene`/`Present`, the engine presents the
swapchain), which is "sigil hosts, engine lends GPU" — the inverse of
the host engine-owns-window; and (b) its `SceneCommand` is frame-level
(`Clear`/`DrawScene`/`Present`), too coarse to be the UI paint API. The real
paint already lives in `renderer` (`emit_scene` → `submit_screen` /
`render_command_scene_offscreen` into a **caller-owned** view).

This is the egui (`eframe` standalone vs `bevy_egui` embedded) / GPUI pattern:
a host-agnostic core, pumped by thin per-host shells.

## Decision

Layer the stack so the **core owns no window and no loop**, and pump it from
two thin shells.

```
┌─ Core (host-agnostic) ──────────────────────────────────────────┐
│ reactive · layout · widgets · text(shaping) ·                    │
│ renderer (paints a Scene into a CALLER-PROVIDED wgpu view)       │
│   already host-free: kernel::Cx, events::EventRouter,            │
│   renderer::{emit_scene, WgpuRenderer, submit_screen,            │
│              OffscreenTarget, SlugTextPipeline}, text::Paragraph  │
└──────────────────────────────────────────────────────────────────┘
        ▲ pumped by                         ▲ pumped by
┌──────────────────────┐          ┌──────────────────────────────────┐
│ sigil winit shell    │          │ the host engine engine (second shell)       │
│  = sigil::launch     │          │  owns window+wgpu+loop;            │
│  (crates/app)        │          │  drives the core inside            │
│  desktop apps        │          │  the host engine::window(...).run(...)       │
└──────────────────────┘          └──────────────────────────────────┘
```

### 1. A host-drivable `UiContext`

Factor the per-frame pipeline that `crates/app/launch.rs` runs today
(reactive scope → drain events → `emit_scene` → paint) into a host-agnostic
`UiContext` that bundles the `Cx`, `EventRouter`, layout, and the renderer
handle. The host calls, in its own loop, at its own cadence:

- `feed(event: InputEvent)` — host pushes normalized input.
- `viewport(w, h)` — host sets the logical size.
- `tick(dt)` — advance reactions / animations.
- `paint(device, queue, view)` — emit the current scene and composite it into
  the host's target (`LoadOp::Load` for embedded; clear for standalone).

The core never acquires a surface or runs an event loop. (`renderer` already
paints into a caller-owned `TextureView` — this property is the linchpin and
it exists today.)

### 2. `InputEvent` normalization

One normalized input vocabulary the core consumes. The desktop shell maps
winit events → `InputEvent`; the host engine maps its `InputState` (keyboard + mouse,
already cursor-unprojected) → the same `InputEvent`. Neither winit nor
the host engine types appear in the core's public surface.

### 3. Public surface

The umbrella `sigil` crate exposes the embedding pieces under an opt-in
feature (`render`): a curated `sigil::ui` (the `UiContext` + `InputEvent`),
`sigil::text` (shaping), and `sigil::render` (the renderer types a host needs
to paint). Desktop apps that only use `sigil::launch` are unaffected; the
feature is off by default.

### 4. `sigil::launch` becomes a thin shell over `UiContext`

`launch` keeps its one-call desktop ergonomics but is re-expressed as: create
a window + surface, create a `UiContext`, pump it from the winit event loop.
No behavioural change for desktop consumers.

### 5. `engine_adapter` — reshape, don't keep as-is

Retire the "sigil drives the engine / engine presents" framing. Keep the one
correct principle (**the engine depends on sigil, never the reverse**). If a
trait boundary is still wanted, it is "the host pumps a `UiContext`", not
"sigil calls `submit_commands` and the engine presents."

## Consequences

- **Desktop unaffected.** `sigil::launch` is the same call; it just sits on
  the shared context.
- **the host engine gets text now, widgets later, same core.** A text-only scene is
  just the first thing the embedded host paints; full widgets/menus paint
  through the identical pipeline with no rework.
- **One paint path.** Both shells go through `emit_scene` + the renderer; no
  divergent render code to keep in sync.
- **Testing.** The headless golden harness already exercises the core without
  a shell, so the embedded path is testable without a window.

## Implementation phases

1. `UiContext` + `InputEvent` in the core (factor out of `launch.rs`).
2. Re-express `sigil::launch` over `UiContext` (prove no desktop regression).
3. Curated `sigil::{ui, text, render}` umbrella surface behind `render`.
4. the host engine: vendor the current sigil, then drive `UiContext` in the bridge —
   text first (Slug `draw_text`), then widget compositing.
5. Reshape/retire `engine_adapter` per §5.
