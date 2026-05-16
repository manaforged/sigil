# DR-018 - DPI and logical-pixel unit system

**Status:** Accepted
**Date:** 2026-04-17
**Depends on:** DR-001 (renderer wgpu), DR-002 (shell winit), DR-016 (app launch), DR-017 (scene paint)
**Closes:** Phase 08b (`docs/plans/phase-08b-dpi-unit-system.md`)
**Unlocks:** Phase 09 reactive recomposition, Phase 11 scene command buffer, Phase 12 scrollview, Phase 14 text in scene

## Context

Before this decision, Sigil had mixed unit semantics. `PhysicalSize`
correctly represented wgpu and OS surface dimensions, while `Point`,
`Rect`, layout styles, hit testing, and scene coordinates were already
logical-pixel-shaped but not named that way. `AppConfig.size` was
documented as physical, pointer positions crossed the shell boundary as
raw winit physical positions, and scale-factor changes carried `f64`
instead of a framework unit type.

That ambiguity would leak into every later phase that sizes, clips,
scrolls, paints text, or virtualizes content. A data-grid row height or
text input caret position must mean the same thing on a 1x monitor and
on a 2x monitor. The renderer still needs physical pixels for surface
configuration, but user code and layout need a stable coordinate space.

## Decision

All user-facing coordinates and sizes are logical pixels. Logical pixels
are the units used by builders, layout styles, event positions, scene
rectangles, and the `viewport` passed to `WgpuRenderer::render_scene`.
Physical pixels remain the HAL boundary unit for wgpu surface creation,
surface resize, and OS window sizes.

`sigil-hal` owns the conversion primitives:

- `LogicalSize { width: f32, height: f32 }` names logical sizes.
- `Scale(f32)` is strictly positive and clamps non-finite or
  non-positive factors to `Scale::ONE`.
- `Scale::to_physical(LogicalSize) -> PhysicalSize` converts logical
  sizes to physical sizes, rounds half-to-even, and clamps each axis to
  at least one physical pixel.
- `Scale::to_logical(PhysicalSize) -> LogicalSize` converts back.
- `Scale::point_to_logical(Point) -> Point` converts shell positions.

`Cx` carries the current window scale through `Cx::scale()` and
`Cx::set_scale(Scale)`. `set_scale` bumps `mutation_generation` through
`Cx::mark_mutated()` only when the raw factor changes, so scale changes
invalidate focus, hit-test, and future layout caches without adding a
second invalidation channel.

`WindowEvent::ScaleFactorChanged` now carries `Scale`. `WinitShell`
tracks the current scale for each live window and converts pointer and
touch positions into logical pixels before buffering `ShellEvent`
values for higher layers.

`sigil::launch` interprets `AppConfig.size` as logical pixels. The
launch loop converts logical viewport sizes to physical sizes when
creating or resizing the wgpu surface, updates `Cx` on scale-factor
changes, and recomputes layout using the logical viewport.

## Rationale

### Why logical default pixels, not physical

Logical pixels are what application authors intend when they write
`fixed_size(720.0, 480.0)` or compare a pointer position against a pane
boundary. Physical pixels would make every layout and hit-test
calculation depend on the monitor the window happens to occupy, forcing
user code to divide by DPI manually and inconsistently.

The GPU surface is the only place physical size is required. Keeping the
physical conversion at the HAL and launch boundaries leaves layout,
events, scene emission, and future widgets in one coordinate space.

### Why `Scale` is on `Cx`, not `LayoutEngine`

Scale changes are application state, not a layout-engine option. Focus
rings, hit-test caches, scroll offsets, text caret geometry, and future
scene caches all need to observe the same invalidation signal. `Cx`
already owns `mutation_generation`, and DR-015 made `mark_mutated()` the
single cache-bump API. Putting `Scale` on `Cx` reuses that contract.

`LayoutEngine` remains a pure logical layout adapter. It consumes a
logical `Size` and attaches logical `LayoutResult` rectangles. That
keeps the eventual incremental-layout work focused on tree/style
mutation instead of monitor DPI.

## Consequences

- Existing builder literals keep their numeric values but are now
  explicitly logical pixels.
- `AppConfig.size` kept its `(u32, u32)` shape for API stability, but
  its interpretation changed from physical to logical.
- `WgpuRenderer::create_surface` and `Renderer::resize` still take
  `PhysicalSize`; wgpu remains physical-pixel-oriented.
- `WgpuRenderer::render_scene` continues to take a logical viewport
  because the scene shader's NDC math expects logical coordinates.
- Pointer and touch positions observed by `EventRouter` are logical.
- Scale-factor changes invalidate `Cx::mutation_generation`, causing
  current caches and future reflow caches to refresh through the same
  path as tree mutation.
- Phase 08b adds a Tier 3 benchmark:
  `scale_change_reflow_1000_nodes`, budget 1,000,000 ns. The committed
  baseline median point estimate is 709,960 ns.

## Alternatives Rejected

| Alternative | Reason |
|---|---|
| Keep raw `f64` scale in events | Lets every caller invent conversion and clamping behavior. |
| Treat app sizes as physical pixels | Makes app layout monitor-dependent and pushes DPI math into user code. |
| Store scale only in `LayoutEngine` | Leaves hit testing, focus, rendering, and future widgets without a shared invalidation source. |
| Convert scene viewport to physical pixels | Breaks the existing WGSL logical-pixel NDC math and would make scene rects and viewport use different units. |
| Panic on invalid scale factors | A transient OS scale report should not crash the app; clamping to 1x is safer and observable through tests. |

## Fix follow-ups

Adversarial review of `f350ceb` found two HIGH-severity gaps in the original Phase 08b implementation: `sigil::launch` created the initial window through the physical-size path after multiplying by `Scale::ONE`, and `WinitShell` forwarded `MouseScrollDelta::PixelDelta` without converting through the current per-window `Scale`. Commit `5b50749` fixes both boundaries.

The fix adds `Shell::create_window_logical` for initial logical window sizing, overrides it in `WinitShell` with `winit::dpi::LogicalSize`, and routes pixel-mode scroll deltas through `Scale::point_to_logical`. The three MEDIUM findings from the same review remain out of scope for this hotfix and are tracked as Phase 08b.2.
## References

- [`docs/plans/phase-08b-dpi-unit-system.md`](../plans/phase-08b-dpi-unit-system.md)
- [`docs/decisions/DR-016-app-launch.md`](DR-016-app-launch.md)
- [`docs/decisions/DR-017-scene-paint.md`](DR-017-scene-paint.md)
- [`crates/sigil/hal/src/size.rs`](../../crates/sigil/hal/src/size.rs)
- [`crates/sigil/kernel/src/cx.rs`](../../crates/sigil/kernel/src/cx.rs)
- [`crates/sigil/shell/src/shell.rs`](../../crates/sigil/shell/src/shell.rs)
- [`crates/sigil/app/src/launch.rs`](../../crates/sigil/app/src/launch.rs)
- [`docs/benchmarks/baseline-sigil-layout.json`](../benchmarks/baseline-sigil-layout.json)

## Kill Criteria

Revisit if:

- A real multi-window API needs independent `Cx` trees with independent
  scale state per window.
- Renderer measurements prove the scene shader should consume physical
  viewport dimensions and scene items should be transformed elsewhere.
- Platform testing shows winit can emit scale changes without matching
  resize information in a way that makes launch-level reconfiguration
  insufficient.
- Sub-pixel snapping or text rasterization requires a dedicated
  device-pixel-ratio abstraction beyond `Scale`.
