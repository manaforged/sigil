# Headless driving and automation

A Sigil app can run headless. `Host` owns the tree without a window, `Host::route` takes synthetic `Event`s, and `Host::accessibility_tree` returns an AccessKit `TreeUpdate`: the same snapshot a screen reader sees. A test or automation script drives the UI through those three calls: read the tree, route events, read it again.

`Host::accessibility_text` renders the tree as indented text, for example `[Role=Button, Name="Increment", Focused=true]`. Node bounds give the click point: scan `tree.nodes` for a node whose `label()` matches and that `supports_action(accesskit::Action::Click)`, then route `PointerPhase::Move`, `Down` (button `Some(MouseButton::Left)`), then `Up`; `Up` without the button does not release a press.

Run `cargo run -p sigil-ui --example headless_driver` for the complete loop: it mounts a counter, prints the snapshot, finds `Increment` by name, clicks it, and prints the snapshot and painted text again.

## Terminal host

The same drive runs without the terminal. `sigil_kittest::Harness::new(tree).size(cols, rows).send(events).text()` mounts, routes, and returns the rendered grid as a string. `Host::grid` advances an existing `Host` by one frame. A driver of a terminal app gets the screen as text on every step.

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

It returns an `Error` whose `kind()` is `ErrorKind::NoAdapter` when no GPU adapter exists, and `Ok(None)` when `SIGIL_ALLOW_NO_GPU=1` is set. `SIGIL_REGEN_GOLDENS=1` rebakes the goldens. `SIGIL_REGEN_GOLDENS=adapter` bakes `<name>.<adapter>.png` for the current GPU adapter. Every test process shares one wgpu device through `sigil_kittest::gpu::shared_device` and `headless_renderer`. See the `sigil-kittest` README for interaction modes and filmstrips.

## Inspect a running app

With the `inspect` cargo feature, an app exposes a local control socket when `SIGIL_INSPECT` is set. `SIGIL_INSPECT=1` binds `127.0.0.1:5761`; any other value is an address to bind. The socket speaks one JSON object per line, one reply per request.

| `op` | Request | Reply |
| --- | --- | --- |
| `a11y::tree` | – | `nodes`: id to `{role, name, description, bounds, children, focused, selected, expanded, toggled, value, numeric}` plus `root` and `focus`. Bounds are in points on the window host and cells on the terminal host. |
| `a11y::snapshot` | – | `text`: the accessibility text dump. |
| `screenshot` | – | Terminal: `text` with the painted grid. Window: `png` (base64) plus `width`/`height` in physical pixels. |
| `layout::audit` | – | `flaws`: a list of `{kind, node, detail}`, where `kind` is a `Flaw` name and `detail` is the `Issue` display text. |
| `surfaces` | – | `surfaces`: keyed surface holes and rects. |
| `settle` | – | `ok` after servicing; `settled` is false while the frame stays dirty or animations run. |
| `click` | `col`/`row` (terminal cells) or `x`/`y` (points), `button` | Moves, presses, and releases the pointer through the normal router. |
| `move`, `down`, `up` | same coordinates | One pointer phase. |
| `widgets::scroll` | coordinates plus `dx`/`dy` in lines | A scroll event. |
| `key` | `key` (a `NamedKey` name or one character), `ctrl`/`shift`/`alt`/`meta` | Key down and up. |
| `type` | `text` | Key events per character. |
| `profile` | `last` (frame count, default 30) | `enabled`, `frames` (index, cause, `wall_ms`, per-phase ms, counters, `backend`), and `summary` (the exit report text). |
| `profile_dump` | `path` (optional) | Writes the Chrome trace JSON now and replies with `path`. Defaults to `SIGIL_PROFILE`'s value or `sigil-profile.json`. |
| `profile_on`, `profile_off` | – | Enable or stop frame collection without restarting. |
| `hud` | `on` (default true) | Draws the frame-HUD overlay: a sparkline, the active backend with any fallback cause, and last-frame phase readout top-right. Terminal host draws cells; the desktop host overlays scene commands at paint. Enabling turns profiling on. |

Requests are serviced between frames on the UI thread, so replies reflect the tree as last laid out. The socket is loopback by convention; the feature is off without the environment variable.

## Profile frames

- `sigil::profile` keys are typed. `span(Phase)` times a phase and
  `count(Counter, n)` records counters. `name()` and `Display` give the trace text. `Frame::counter(Counter)` reads a counter; `Frame::phases` and `Frame::counts` hold `PhaseTime` and `CounterValue` pairs.
- `profile::begin_frame(BackendStatus)` opens a frame for one host. A
  `BackendStatus` is a `Backend` (`Gpu`, `GpuSoftware`, `Cpu`, `Terminal`) and a `FallbackCause` (`None`, `NoAdapter`, `DeviceError`, `ForcedByEnv`, `UnknownEnv`). `BackendStatus::TERMINAL` is the terminal host's status. `Frame::backend` holds the status of the host that drew that frame, so a window host and a terminal host in one process keep separate statuses.
- `profile::interact(Cause)` marks the input that caused a frame.
  `Cause` is `Idle`, `Pointer`, `Hover`, `Key`, `Scroll`, `Ime`, `Resize`, `A11y`, `Expose`, or `Input`. When several inputs land in one frame, the highest `Cause::rank()` wins. `Frame::cause` holds it.
