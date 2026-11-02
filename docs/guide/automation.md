# Headless driving and automation

A Sigil app can run headless. `Host` owns the tree without a window, `Host::route` takes synthetic `Event`s, and `Host::accessibility_tree` returns an AccessKit `TreeUpdate`: the same snapshot a screen reader sees. A test or automation script drives the UI through those three calls: read the tree, route events, read it again.

`Host::accessibility_text` renders the tree as indented text, for example `[Role=Button, Name="Increment", Focused=true]`. Node bounds give the click point: scan `tree.nodes` for a node whose `label()` matches and that `supports_action(accesskit::Action::Click)`, then route `PointerPhase::Move`, `Down` (button `Some(MouseButton::Left)`), then `Up`; `Up` without the button does not release a press.

Run `cargo run -p sigil-ui --example headless_driver` for the complete loop: it mounts a counter, prints the snapshot, finds `Increment` by name, clicks it, and prints the snapshot and painted text again.

## Terminal host

The same drive runs without the terminal. `sigil_kittest::Harness::present(tree, cols, rows, events)` mounts, routes, and returns the rendered grid as a string. `Host::grid` advances an existing `Host` by one frame. A driver of a terminal app gets the screen as text on every step.

## Golden images

`sigil-kittest` is the PNG harness for dependents. It mounts a widget in a `Host`, renders the scene offscreen with wgpu, and compares pixels against a committed golden:

```rust,no_run
# use sigil::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let Some(img) = sigil_kittest::Harness::render(|| button("Save"))
    .size([200, 60])
    .hit((10.0, 10.0))
    .image()?
else {
    return Ok(());
};
sigil_kittest::Golden::new("save").assert_image(&img);
# Ok(())
# }
```

It returns `Err(Error::NoAdapter)` when no GPU adapter exists, and `Ok(None)` when `SIGIL_ALLOW_NO_GPU=1` is set. `SIGIL_REGEN_GOLDENS=1` rebakes the goldens. `SIGIL_REGEN_GOLDENS=adapter` bakes `<name>.<adapter>.png` for the current GPU adapter. Every test process shares one wgpu device through `sigil_kittest::gpu::shared_device` and `headless_renderer`. See the `sigil-kittest` README for interaction modes and filmstrips.
