# Indicators

Read-only status visuals. They paint state; except for chip with `.activate()`, none of them take input, and they do not implement `StateHost`. State closures are a compile error by design.

## chip

`chip(label, Tone) -> ChipBuilder`: a 20 px `text-xs font-medium` label in a tinted `rounded-full` pill. Use it for a status tag or a count. The label renders as written; write it in sentence case.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
chip("Stable", Tone::Success);
chip("3", Tone::Error);
# };
```

- `.tone(..)` takes a `Tone`, a `Signal<Tone>`, or a closure; a signal or closure retints the chip when it changes.
- `.activate(Fn())` turns the chip into a button: `Role::Button`, pointer release and Enter fire the closure. Without it the chip is `Role::Label` and inert.
- `.layout(LayoutStyle)` returns a `NodeBuilder` for manual layout.

The tone palette is fixed: `Neutral`, `Primary`, `Info`, `Success`, `Warn`, `Error`.

## dot

`dot(Tone) -> DotBuilder`. A status dot: an outer ring at scale step 11 with an inner fill at step 9. `.size(Size)` picks the extent: Compact 8 px, Default 12 px, Wide/Grow 16 px. Not hittable, not focusable.

## gauge

`gauge(Signal<f32>) -> GaugeBuilder`. A labeled 160×8 px meter with a proportional fill. `Role::ProgressIndicator`; publishes the value over 0..max.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let cpu = signal(0.42_f32);
gauge(cpu).max(1.0).label("CPU").warn(0.2);
# };
```

- `.max(f32)` sets the denominator (default 1.0; values below `f32::EPSILON` are clamped to it).
- `.label(impl Into<TextContent>)` draws text before the track and becomes the accessible name (default "Gauge").
- `.warn(threshold)`: a ratio, not an absolute value. Below the threshold the fill paints `warn-9`; at or above it paints `accent-8`. The default (no call) always paints `accent-8`.

## progress

`progress(Signal<f32>, max: f32) -> ProgressBuilder`. An 8 px fill bar. `Role::ProgressIndicator`.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let done = signal(30.0_f32);
progress(done, 100.0);
# };
```

- `.height(px)` and `.width(px)` size the track; the default fills the parent width.
- `.indeterminate()` ignores the value as a fraction and treats the signal as a phase in 0..1: a 30%-width segment slides across the track. Drive the signal from a timer or task to animate it.

Fills are expressed as `flex-grow` units, so the bar tracks layout width exactly.

## spinner

`spinner() -> NodeBuilder`. An indeterminate activity indicator: a braille glyph that advances one frame every 80 ms. `spinner_frame(tick)` returns the glyph for one frame, so you can draw the same animation in your own text node.

## States, keys, and accessibility

Indicators have one visual state each: the visual catalog captures only the default state. They take no keys, except a chip with `.activate()`.

| Widget | Keys | Role | Name source |
| --- | --- | --- | --- |
| `chip` | Enter with `.activate()`, else none | `Label`, or `Button` with `.activate()` | The label |
| `dot` | none | `GenericContainer` | None. Put the meaning in a nearby label. |
| `gauge` | none | `ProgressIndicator` | A static `.label()`, else "Gauge" |
| `progress` | none | `ProgressIndicator` | `.label()`, else "Progress" |
| `spinner` | none | `ProgressIndicator` | `.label()` on the returned `NodeBuilder`, else "Loading" |

## Terminal notes

- A gauge or progress track paints as a row of cells. The fill ends on a half-block glyph when it stops inside a cell.
- A dot paints as a colored cell. With `NO_COLOR` the tone is lost, so do not use a dot as the only signal.
- The spinner glyph is plain text, so it paints the same in a terminal. The host builds a frame only when the spinner interval is due.

## Example

Run it with `cargo run -p sigil-ui --example widget_indicators`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_indicators.rs}}
```
