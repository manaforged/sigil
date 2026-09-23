# Indicators

Read-only status visuals. They paint state; except for chip with
`.activate()`, none of them take input, and they do not implement
`StateHost` — state closures are a compile error by design.

## chip and badge

`chip(label, Tone) -> ChipBuilder` — a small `text-xs` label in a
tinted `rounded-sm` frame. `badge(label, Tone)` is the same builder
with `.pill()` applied: `rounded-full` instead of `rounded-sm`.

```rust,ignore
chip("stable", Tone::Success);
badge("3", Tone::Error);
```

- `.tone(Tone)` retints statically; `.tone_signal(Signal<Tone>)`
  retints reactively.
- `.activate(Fn())` turns the chip into a button: `Role::Button`,
  pointer release and Enter fire the closure. Without it the chip is
  `Role::Label` and inert.
- `.layout(LayoutStyle)` returns a `NodeBuilder` for manual layout.

The tone palette is fixed: `Neutral`, `Primary`, `Info`, `Success`,
`Warn`, `Error`.

## dot

`dot(Tone) -> DotBuilder`. A status dot: an outer ring at scale step 11
with an inner fill at step 9. `.size(Size)` picks the extent —
Compact 8 px, Default 12 px, Wide/Grow 16 px. Not hittable, not
focusable.

## gauge

`gauge(Signal<f32>) -> GaugeBuilder`. A labeled 160×8 px meter with a
proportional fill. `Role::ProgressIndicator`; publishes the value over
0..max.

```rust,ignore
let cpu = signal(0.42_f32);
gauge(cpu).max(1.0).label("CPU").warn(0.2);
```

- `.max(f32)` sets the denominator (default 1.0; values below
  `f32::EPSILON` are clamped to it).
- `.label(impl Into<TextContent>)` draws text before the track and
  becomes the accessible name (default "Gauge").
- `.warn(threshold)` — a ratio, not an absolute value. Below the
  threshold the fill paints `warn-9`; at or above it paints `accent-8`.
  The default (no call) always paints `accent-8`.

## progress

`progress(Signal<f32>, max: f32) -> ProgressBuilder`. An 8 px fill bar.
`Role::ProgressIndicator`.

```rust,ignore
let done = signal(30.0_f32);
progress(done, 100.0);
```

- `.height(px)` and `.width(px)` size the track; the default fills the
  parent width.
- `.indeterminate()` ignores the value as a fraction and treats the
  signal as a phase in 0..1: a 30%-width segment slides across the
  track. Drive the signal from a timer or task to animate it.

Fills are expressed as `flex-grow` units, so the bar tracks layout
width exactly.
