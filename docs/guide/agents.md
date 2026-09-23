# Driving Sigil from an agent

A Sigil app can run headless. `Host` owns the tree without a window,
`EventRouter` takes synthetic `ShellEvent`s, and
`Host::accessibility_tree` returns an AccessKit `TreeUpdate` — the same
snapshot a screen reader sees. An agent drives the UI through those
three calls: read the tree, route events, read it again.

`sigil::a11y::snapshot` renders the `TreeUpdate`
as indented text — `[Role=Button, Name="Increment", Focused=true]` —
and node bounds give the click point: scan `tree.nodes` for a node whose
`label()` matches and that `supports_action(accesskit::Action::Click)`,
then route `PointerPhase::Move`, `Down` (button
`Some(MouseButton::Left)`), then `Up`; `Up` without the button does not
release a press.

Run `cargo run -p sigil-gui --example agent_driver` for the complete
loop: it mounts a counter, prints the snapshot, finds `Increment` by
name, clicks it, and prints the snapshot and painted text again.

## Terminal host

`sigil-tui` exposes the same drive without the terminal:
`sigil_tui::present(tree, cols, rows, events)` mounts, routes, and
returns the rendered grid as a string. `sigil_tui::step` advances an
existing `Host`/`EventRouter` pair by one frame. Agents that drive a
terminal app get the screen as text on every step.

## Golden images

`sigil-kittest` is the PNG harness for dependents. It mounts a widget
through `EventRouter`, renders the scene offscreen with wgpu, and
compares pixels against a committed golden:

```rust,ignore
let Some(img) = sigil_kittest::render_widget_to_png(
    |cx| cx.add(button("Save")),
    sigil_kittest::InteractionMode::Default,
    [200, 60],
    (10.0, 10.0),
) else {
    return; // no adapter
};
sigil_kittest::assert_png_matches_golden(&img, &sigil_kittest::golden_path!("save"), 0.5);
```

It returns `None` when no GPU adapter exists. `SIGIL_REGEN_GOLDENS=1`
rebakes, and `SIGIL_REQUIRE_GOLDENS=1` turns the skip into a failure.
See the `sigil-kittest` README for interaction modes and filmstrips.
