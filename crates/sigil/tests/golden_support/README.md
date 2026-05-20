# Visual-golden PNG harness

This directory hosts the helper module backing
`crates/sigil/tests/visual_goldens.rs`. The harness drives a widget into one
of eight interaction modes — five settled (`Default`, `Hover`, `Pressed`,
`Focused`, `Disabled`) and three frame-aware (`HoverAfterFrames`,
`PressedAfterFrames`, `FocusedAfterFrames`) — renders the resulting node
tree through `WgpuRenderer::render_command_scene_offscreen`, reads pixels
back, and compares against a committed PNG in
`crates/sigil/tests/goldens/`.

## Why committed PNGs

`tests/widget_states.rs` already asserts theme-colour rectangles for each
(widget, state) combo programmatically — that proves rendering is
consistent across runs but produces no artifact a human can eyeball. The
PNG goldens commit one image per widget × state so a reviewer can open
them in any image viewer and confirm the widget renders correctly.

## API

```rust
use sigil::golden_support::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png,
};

let Some(img) = render_widget_to_png(
    |cx| cx.add(button("Save")),
    InteractionMode::Default,
    [200, 60],
    (10.0, 10.0),         // hit-point used for hover/pressed
) else {
    println!("no GPU adapter — skipping");
    return;
};
assert_png_matches_golden(&img, &golden_path("button_default"), 0.5);
```

The harness returns `None` when no wgpu adapter is available (CI without
a GPU). Tests skip cleanly in that case.

`InteractionState` is not an external lever in sigil — each widget owns
its own `crate::interaction::InteractionState` and drives state through
`EventRouter` pointer/key events. The harness mirrors that pathway, so
`hit_point` must land inside the widget's hit-test surface for Hover and
Pressed to take effect. `Disabled` is driven through the widget builder
itself (e.g., `button("Save").enabled(false)`) — the harness skips events
in that mode.

### Animated modes (W5.0a)

`HoverAfterFrames`, `PressedAfterFrames`, and `FocusedAfterFrames` drive
the same pointer / focus event as their non-AfterFrames counterpart, then
advance `sigil_animation_runtime::tick(dt)` for `frames` iterations so
animator-driven style atoms resolve to the post-tick value before
`emit_scene` captures. The canonical `dt` is `1.0 / 60.0`.

```rust
let img = render_widget_to_png(
    |cx| cx.add(button("Save")),
    InteractionMode::HoverAfterFrames { frames: 6, dt: 1.0 / 60.0 },
    [160, 56],
    (74.0, 22.0),
);
```

`frames: u32::MAX` is a sentinel meaning "tick until every active
animator settles." A safety cap (`SETTLE_SAFETY_CAP = 600` frames — 10 s
of simulated time at 60 Hz) guards against runaway animators. A widget
that retargets on every tick will trip the cap and panic with a clear
message rather than stalling the test.

`InteractionMode::tag()` returns the same lowercase tag for both the
settled and `*AfterFrames` variants (e.g., both `Hover` and
`HoverAfterFrames` tag as `"hover"`), so call sites that bake animated
goldens should append their own `_animated` suffix to the golden stem.

## Regenerating goldens

After an intentional rendering change:

```pwsh
$env:SIGIL_REGEN_GOLDENS = "1"
cargo test -p sigil --test visual_goldens
Remove-Item Env:\SIGIL_REGEN_GOLDENS
```

Or in bash:

```bash
SIGIL_REGEN_GOLDENS=1 cargo test -p sigil --test visual_goldens
```

When the env var is set, the harness writes each captured PNG to its
golden path instead of asserting. **Open every regenerated PNG in an
image viewer before committing** — confirm the widget actually renders.
The brief's anti-pattern: do not commit placeholder / empty / black
PNGs. If a widget renders incorrectly under the harness, that is the
bug — flag it; do not bake junk goldens.

## Tolerance

`assert_png_matches_golden(..., max_delta_pct)` accepts a small budget
of differing pixels to accommodate GPU-driver tolerances (rounding in
sRGB conversion, anti-aliasing edge cases). A pixel counts as
"different" when any RGBA channel differs by more than 2 units. The
default budget is 0.5%.

If a real visual regression sits below the budget, increase the
specificity of the check rather than tightening the budget across the
board.

## Why a clear black background

The offscreen path clears to `(0, 0, 0, 1)`. Widgets paint themed
surfaces on top, so the contrast is high. This makes eyeball review
trivial; if the background ever needs to be themed, expose a parameter
in `render_widget_to_png` rather than per-widget hard-coding.
