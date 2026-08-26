# Profiling Sigil

The profiler follows the same model Chrome, Flutter DevTools, and
React DevTools use:

1. Name the input that woke the frame.
2. Time that frame as wall clock (input → next present).
3. Split the time into layout / emit / GPU and count what painted.

Off until you turn it on. When it is off, each call is one atomic
load.

## Record a live app

```text
SIGIL_PROFILE=1 cargo run --profile profiling -p workbench
```

Use the app: click, type, hover, scroll. Quit. The process writes
`./sigil-profile.json` and prints a table on stderr.

Open the JSON in [ui.perfetto.dev](https://ui.perfetto.dev). You get:

- **ui** thread: `effect`, `layout`, `emit`, `a11y`
- **gpu** thread: `gpu`
- **input** instants (`pointer`, `hover`, `key`, `scroll`, …)
- **flows** from the input to the frame that presented it
- **counters**: `scene.cmds`, `scene.glyphs`, `text.measure`, …

Write somewhere else with `SIGIL_PROFILE=/tmp/sigil-profile.json`.

`--profile profiling` keeps line-table debuginfo on a release build
so samply / Instruments can resolve symbols.

## Read the table

```text
sigil-profile  (ms)
phase                 p50      p95      max     last
effect              0.120    0.400    1.200    0.130
layout              0.200    0.400    4.500    0.210
emit                1.400   14.200   18.000   14.100
gpu                 0.600    1.100    2.000    0.590
frame               2.400   16.900   20.000   16.800

interactions     n      p50      p95      max     last
pointer          4     3.100    8.200   12.000    3.000
hover           40     2.100   16.800   18.000   16.800

counts
scene.cmds        4200     4200     4200     4200
scene.emit           1        1        1        1
text.measure        12       40       80       12

bottlenecks (over 16.67ms)
  #12   hover        16.8ms  emit=14.1  cmds=4200
12 frames, 8 over 16.67ms
```

`emit` skips nodes whose layout box is fully outside the window
or an overflow clip (plus 64px overscan). A 500-row list in an
800px pane should show tens of `scene.cmds`, not hundreds. If
hover still emits the whole tree, a parent is `overflow: visible`
and flex-shrink is packing every row into the viewport.

The first eight over-budget interaction frames also print live:

```text
sigil-profile: frame 12 hover 16.8ms emit=14.1 cmds=4200
```

## Side-by-side with GPUI `data_table`

A browser tab cannot host two live wgpu windows. Run the same 10k × 24
column table as two native processes, then drop traces on
[`showcase/compare/data-table.html`](../../showcase/compare/data-table.html).

```text
./scripts/compare-data-table.sh
./scripts/bench-data-table.sh
SIGIL_PROFILE=./sigil-data-table.json cargo run --profile profiling -p data-table
```

GPUI 0.2.2 has no Chrome-trace profiler. Record it with samply or
Instruments on the same scroll gesture.

## Headless sample

```text
cargo run -p sigil-ui --example profile_frame --release
```

Builds a 500-row tree, drives 60 hover/scroll frames through layout
+ emit, and writes the same table. No window.

## Causes `launch` records

| Cause | When |
| --- | --- |
| `pointer` | Pointer down or up |
| `hover` | Pointer move |
| `key` | Key event |
| `scroll` | Wheel / trackpad |
| `ime` | IME composition |
| `resize` | Window resize or scale change |
| `a11y` | AccessKit action |
| `animation` | Spring / tween advanced |
| `interval` | `cx.interval` fired |
| `expose` | OS redraw of an unchanged scene |
| `idle` | No cause recorded |

A click in the same frame as a move upgrades `hover` to `pointer`.

## Phases

| Phase | What it covers |
| --- | --- |
| `effect` | Input drain + animation tick |
| `layout` + `layout.sync` / `solve` / `attach` | Compute |
| `paint` | Full paint, including emit + GPU |
| `emit` | Walk the tree into a command scene |
| `gpu` | `render_command_scene` (Perfetto gpu track) |
| `a11y` | AccessKit publish |

## Counters

| Name | Meaning |
| --- | --- |
| `scene.emit` / `scene.replay` | New scene vs last-frame replay |
| `scene.cmds` / `scene.glyphs` / `scene.rects` / `scene.images` | Command-buffer mix |
| `gpu.rects` / `gpu.glyphs` / `gpu.sprites` / `gpu.layers` | What the GPU walker submitted |
| `text.measure` | Paragraph measure calls |
| `path.raster` | CPU path rasters that missed the intern cache |
| `layout.hit` / `layout.miss` | Layout cache |
| `layout.full` / `layout.incr` / `layout.resize` | Which compute path ran |

## Author spans

```rust,ignore
use sigil::profile;

let _work = profile::span("catalog.sort");
profile::interact("pointer");
profile::count_add("rows.sorted", rows.len() as u64);
```

`sigil::launch` already calls `begin_frame` / `tick_frame` and dumps
on exit. Hosts that drive their own loop call those at the start and
end of each frame, `interact` when they drain input, and
`profile::dump("sigil-profile.json")` on quit.

## Benches

```text
cargo bench -p sigil-ui --bench frame_phases -- --test
cargo bench -p sigil-ui --bench frame_phases
```

Frame budget is `sigil::profile::FRAME_BUDGET_NS` (16.67 ms). That is
the native click/type bar. `INTERACT_BUDGET_NS` (200 ms) is Chrome INP
only — the report names it as such. Do not treat 200 ms as good for a
wgpu click.

Headless Increment of wrapping `text(derived)` on a 40-row column is
`layout.incr`, not `layout.full`. Prove with:

```text
cargo nextest run -p sigil-layout --release --test profile derived_text_increment
```

That is layout only, not a presented wgpu frame. Native click bar stays
`FRAME_BUDGET_NS` (16.67 ms).

Headless click-to-present (Increment on the hello-counter tree:
`signal.set` + layout + scene emit, not a presented wgpu frame):

```text
cargo test -p sigil-ui --release --test click_present -- --nocapture
```

The test prints `median=` / `mean=` / `cv=`. Label that number
**interaction**, not construct. It must stay under `FRAME_BUDGET_NS`.

Quoted (Apple M2 Max, rustc 1.98.0, `--release`, n=64): median 43 µs,
mean 45 µs, cv 0.109. Not a presented wgpu frame.
