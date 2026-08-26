# strays — friction log

## 2026-08-19 — first build on `sigil-tui`, and the fixes it forced

A terminal inventory of every listening process on the machine, built to
find out what a keyboard-first TUI needs from Sigil that a desktop window
does not. The app is small on purpose; the point is what it broke.

Ten defects and one wrong assumption came out of it, all fixed in this
branch. Regression tests live in `crates/tui/tests/host.rs` — the example is
a dogfood, not a gate.

An earlier version of this file said "each with a regression test". That was
not true, and an adversarial review proved it by reverting each fix and
watching the suite stay green. What each test is actually worth is recorded
against the defect, and the review is summarised at the end.

## Fixed

### 1. A chip cell painted one row below its data

`raster` placed a glyph at `floor(rect.origin.y / CELL)`. A chip's chrome
offsets its label ~4 px inside the row, which is half of an 8 px cell, so
the glyph crossed into the next row while the plain labels beside it
stayed put. Rows appeared to show their neighbour's state:

```
PID    STATE    WHERE
 1111            ?          <- blank
 2222   system   a          <- 1111's state
 3333   STRAY    b          <- 2222's state
                 fresh      <- 4444's state, below the table
```

Measured, same row: label text at `y=29.00`, chip text at `y=32.00`,
row pitch 8. `floor` put them in cells 3 and 4.

Fixed by giving `Painted` the clip box it paints inside, then treating a
box no taller than one cell as the row itself. Taller boxes fall back to
the glyph's own middle. Regression: `a_chip_cell_paints_on_its_own_row`.

The first attempt used the clip's top edge and broke the sticky header —
a clipped first body row starts mid-cell, so the header and row one
collided. The clip's centre is stable against that.

### 2. Fixed columns did not clip

`ColWidth::Fixed` sized the cell but did not bound what it painted, so a
long value overprinted its right-hand neighbour: `49159,573116h47m414M`
is `PORT`, `AGE`, and `MEM` on top of each other.

The table was never at fault — `table/columns.rs:215` and
`table/row.rs:104` already set `Overflow::Hidden`, and desktop honours it
through `emit_scene_clipped`. The TUI's `push_painted` walk simply had no
clip concept, so it dropped the setting.

Fixed by propagating the clip down that walk, the same rule desktop uses.
`raster` now cuts a text run at the clip's right edge. Regression:
`a_wide_cell_clips_at_the_column_edge`.

The example carried its own `clip()` helper until this landed. It is gone
— cutting text is the column's job, not every caller's.

### 3. Buttons painted as slabs of solid block

```
█████████   ██████
█████████   ██████
█Refresh█   █Kill█
```

`raster` filled any toned box that had no text of its own with `█`, which
is how a progress fill and a chart bar are drawn. A button's surface is
also a toned box with no text — its label is a child node.

The distinction is content: a bar holds nothing, a button holds its
label. `Painted::bare` now carries it, and the block fill is gated on it.
Regression: `a_button_surface_is_not_a_block_slab`, alongside the
existing progress and chart tests which still demand their bars.

### 4. Nothing sized to a terminal line

`Host::CELL` is 8 px and every control took its height from the theme's
desktop metrics: `control-height-default` is 28 px, so a button reserved
four lines. `Density::Compact` is 22 px, still three.

The heights were never the widget's to begin with — `button()` reads
`theme.control_height(Size::Default)`, and so does everything else. So
this is one hook, not a catalogue-wide edit: `Theme::grid(cell)` sets a
thread-local cell size that the theme constructors apply, collapsing
control heights, padding, and gaps onto the grid and dropping corner
radius. `sigil-tui` sets it once in `run` and `present`; desktop is
untouched because the grid defaults to zero.

Regression: `a_control_stands_one_line_tall`.

### 5. A filled table slid its header under row one

With `viewport_fill()` the header and the first row painted on the same
line. The measurement, from the app's own tree:

```
header y=20.030 h=8.00
row 1  y=27.745          <- 7.714 px below a box 8 px tall
```

The header row set a height but stayed shrinkable, so a growing body took
the difference back out of it and the two boxes overlapped. `viewport_fill`
also lands rows on fractional origins (`y=8.52`, not `9.00`) because it
divides the available height evenly, so glyph placement now snaps a
single-cell box to the nearest line rather than flooring it.

Fixed in `table/header.rs`: the sticky header does not shrink. Regression:
`a_filled_table_keeps_its_header_off_row_one`.

### 6. No periodic tick for app code

The loop already ticked at 33 ms to drive animations and app code could
not reach it, so a process monitor needed a keypress to notice a process
had exited. `sigil_tasks::spawn` wants an async runtime, which a binary
that needs "re-scan every two seconds" should not have to stand up.

`sigil_tui::every(period, f)` is a countdown on the frame tick. A headless
`step` advances it exactly as a live frame does, so the test drives the
clock without waiting on one. Regression:
`a_timer_comes_due_on_the_frame_clock`.

strays now re-scans every two seconds, and `r` is only there for when you
do not want to wait.

### 7. Colour never reached the terminal

`Cell` carried a tone from the paint walk and `write_row` printed glyphs
only. Every badge, accent, and `Tone::Warn` rendered as plain text — a
`badge(label, Tone::Warn)` whose whole job is to be yellow was
indistinguishable from a label. A tone-only change still tripped the row
diff, so the host rewrote bytes identical to what was already on screen.

A tone means two different things depending on what carries it: a box
paints a surface, a text node paints ink. `Cell` now holds `fg` and `bg`
separately, `write_row` emits one SGR pair per run rather than per cell,
and an untoned row emits none at all so an unstyled app keeps the palette
the user chose.

Writing that test found a second defect: a toned text node filled its own
background with its own tone, so the glyph sat on its own colour and
disappeared. Only a box paints a surface now. Regressions:
`a_toned_glyph_carries_its_colour_to_the_terminal`,
`toned_text_does_not_paint_its_own_background`, `an_untoned_row_sets_no_colour`.

### 8. Every glyph was one column wide

`raster` advanced one cell per `char`, and the cell text measure sized a
box by `chars().count()`. A CJK glyph occupies two columns and a combining
mark occupies none, so one wide character slid the rest of its line — and
every column after it — one place left.

Both now measure display width. A wide glyph claims its second column with
a continuation cell that nothing prints, because printing a filler would
push the line right again. Regression: `a_wide_glyph_keeps_the_next_column`.

### 9. The terminal saw six colours, not the theme

`Painted` carried a `Tone` — a six-value enum — while the theme holds full
Radix scales and the window host resolves real colours from them. A badge
came out as the nearest of eight ANSI colours, and a `text-gray-11` class
reached the terminal as nothing at all.

Both hosts now resolve through the same code: `render_style_for` and
`text_style_for` moved out of `sigil-renderer` into `sigil_style::resolve`,
so a terminal frame and a window frame agree by construction rather than by
two implementations staying in step. The writer emits 24-bit colour when a
style resolved one and falls back to the terminal's palette when it did not,
which is what an unthemed widget and a user's own scheme both want.

Resolving in the terminal for the first time found a latent defect: `chart()`
asked for `border-line`, and no `line` token exists, which panics in a debug
build. The window host would have done the same.

### 10. Replacing the rows did not repaint the table

Found by review, not by me, and the worst one here: `table/row.rs` read its
row data with `peek()`, which reads a signal without subscribing to it. The
cells subscribed to the column layout and nothing else, so `replace_rows`
changed the data and the screen kept the first frame.

This app rescans every two seconds. Its entire reason to exist did not
repaint, and no test noticed because every test built its table at the state
it asserted. It predates this branch — the bug is as old as the widget — but
it is the one that made the app pointless. `crates/tui/tests/host.rs`
`replacing_rows_repaints_the_table` fails against it.

## Corrected

**Sigil does have key bindings. The first version of this log said it did
not, and that was wrong.**

`NodeEventsExt::key(shortcut, handler)`, `key_any`, and `key_capture` all
exist and work. `strays` binds bare `r` and `k` through `key_capture` on
the root, which fires before the focused widget sees the key. Proven by
`keys::a_bare_key_reaches_the_root` in `src/main.rs`.

The reason it read as missing: those three are the only handlers on the
trait without an `on_` prefix, next to `on_scroll` and `on_ime`. Grepping
`on_key` finds nothing and the trait is long enough that reading it end to
end is not the first instinct. The trait is inconsistent with itself
(`pointer_leave` and `focus_in` also skip the prefix), so this is a naming
question for the whole surface rather than something to patch for keys
alone. Adding an `on_key` alias would leave two names for one thing.


## Performance

Measured with `cargo bench -p sigil-tui --bench frame` on an idle machine
(load average 3): a 200-row table on a 120x40 grid. Criterion's interval,
not a point estimate — earlier numbers in this file were quoted to three
figures off benches whose spread was a third of their mean.

| | value |
| --- | --- |
| idle frame, paint to flush | 40.3 - 40.6 us |
| churn frame, every row replaced | 1.137 - 1.145 ms |
| paint walk | 38.0 us |
| raster | 12.2 us |
| layout | 1.90 us |
| write, unchanged | 8.7 us |
| write, full redraw | 58.1 us |
| allocations per frame | **1 (1024 B)** |

Read the last row carefully. `paint_into` and `raster_into` each allocate
nothing, and this file previously rounded that to "0 allocations per frame".
The frame allocates: `Host::layout` takes 1 KiB every time, and the bench
prints it. Two phases at zero is not a frame at zero.

The paint walk got there in three steps. Allocation was one `String` per
text node per frame, because `resolve()` clones even fixed text and the walk
rebuilt its vector; `paint_into` now overwrites slots so each node keeps its
text buffer. That was worth 4 us. `raster` was bounds-checking the whole
grid per character and now takes the row slice once per run — worth about
half its time. Then colour arrived and cost more than both together, because
resolving a node means applying its style atom slices through the engine,
so each paint slot now remembers what its node resolved to and what that
answer depended on.

Two guesses missed before a measurement found it: the duplicated warning
pass over each atom slice was worth 2 us, and the duplicated extension
lookups nothing at all. Stubbing the resolver out is what split the number.

### An idle app should cost nothing to show

A terminal app on screen and doing nothing still painted, rasterized 4800
cells, and diffed the whole grid thirty times a second to conclude that the
screen was already right. The walk now reports whether any slot moved, and
`Screen` skips the raster and the write when none did. An idle frame sends
the terminal no bytes.

Getting the comparison honest took two passes. Comparing whole records made
every parent report movement, because `bare` is only known after a node's
children are walked. And comparing whole records is more work than needed —
a style the cache answered is identical by construction, so only what layout
can move is worth checking.

### A churn frame is thirty times an idle one

Replacing every row costs 1.14 ms against 40 us. That number did not exist
until the review, because the table could not repaint at all (see below), so
there was nothing to measure. It is the honest ceiling for this app: a
two-second refresh spends about a millisecond, which is fine, and it is
where the style cache is worth nothing because every node genuinely changed.

### The scan was the real cost

The frame was never the problem. A refresh was 42 ms: sockets 1.8 ms, a
`ps -axo` fork 32 ms, and `sysinfo` refreshing every process on the machine
8 ms to read a dozen.

Two mistakes, both mine. Replacing the `unsafe` `libc` calls with one `ps`
traded three function calls for a fork, and a fork costs about 20 ms here
whether it lists every process or fifteen. And the scan refreshed every
process to read the few that hold a port.

A process's group and controlling terminal do not change, so `Scanner` learns
them once per pid and forks only for a pid it has not seen; `sysinfo`
refreshes only the pids holding a port, and only the fields the table shows.

Repeat scan measures **2.5 - 4.3 ms** across runs, in a debug build, on a
machine with eighteen listeners. An earlier version of this file published
1.77 ms from a single release-build run; that number is not reproducible and
should not have been quoted. The guard,
`cost::a_repeat_scan_stays_under_a_frame`, asserts only that a repeat scan
fits in a frame — nine times the measured value, so it defends the shape of
the win and not its size. The first scan still costs 25 - 30 ms and runs
synchronously from the tree builder, so mount blocks for it.

## What an adversarial review found

Four reviewers were set on this work with instructions to break it: one on
the benchmarks, one on test quality, one on the style cache, one comparing
every claim to its evidence. They found more than the work itself did.

**A stale cache, demonstrated.** `render_style_for` reads
`binding.reactive_channels()`, whose contents live in a `Cell` written
without touching the node. The cache key did not name them, so a reactive
class change served the old colour — and because the walk then reported
nothing moved, the terminal was never even sent the difference. Table row
selection is `class_when`, so selecting a row was invisible. The key now
carries each atom slice's identity (a slice is a fat pointer; its address
and length name it exactly), which is the same trick the layout cache uses,
documented in `sigil-layout` for this exact trap.

**Tests that guarded nothing.** Reverting the clipping fix left every test
passing, because the assertion was `grid.contains("tail")` and the corrupted
output still contains "tail". Reverting the header-shrink fix left its own
named test passing. Hard-wiring the walk to report "nothing moved" — which
freezes the terminal after one frame — left all 31 tests green. Those are
rewritten or replaced; the ones that survived reversion are noted below.

**Numbers that did not reproduce.** "0 allocations per frame" was two phases
measured in isolation, not a frame. "raster 14.6 -> 7.9 us" quoted a state
that was reverted when colour landed; shipped raster is 12.2. The scan's
1.77 ms came from one release run and measures 2.5 - 4.3 ms. And the bench
named `steady` never flushed, so it took a branch through `Screen` the live
loop leaves after frame one.

**A benchmark measuring a tree that could not change.** The idle numbers were
real but hollow: the table could not repaint (defect 10), so "nothing moved"
was not a diff concluding correctly, it was a widget that could not move.
That is fixed, and `churn/200-rows` now measures the other side.

Tests that survived reversion, and are worth keeping: the block-slab guard,
the one-line control height, the toned-text background, the frame-clock
timer, the four verdict tests, and the bare-key capture.

## What a real app still needed

The defects above are about painting correctly. These are about being usable
for something someone depends on.

### 11. The terminal host dropped every mouse event

`Event::Mouse(_) => {}`. Mouse reporting was never even enabled, so the
terminal sent nothing, and every pointer-driven behaviour in the widget set
was dead on this host: hover, click-to-select, wheel scrolling, header
click-to-sort, column drag. A table you cannot scroll or click is not a
table, and the router already accepted `ShellEvent::Pointer` and `::Scroll` —
nothing had to be designed, only carried.

A cell maps to the point at its **centre**, not its corner. A corner sits
exactly on a boundary, and the table's one-pixel border was enough to put a
click in the row above. That cost one failing test to find. Regressions:
`a_click_selects_the_row_under_it`, `the_wheel_scrolls_the_body`.

### 12. A panic left no trace

The default hook prints to a screen that is still in raw mode and still
showing the alternate buffer. The message goes into a buffer that is torn
down a moment later, so an app that crashes leaves a blank terminal and no
reason. The host now restores the terminal before the panic prints, chaining
to whatever hook was already installed.

### 13. Truecolor was assumed

24-bit escapes went out unconditionally. A terminal without truecolor renders
them as garbage or ignores them. `COLORTERM` is read once, and a resolved
colour comes down to the nearest slot in the 256-colour cube when the
terminal has not advertised it.

### 14. The app mounted, then froze

`strays` built its state inside the tree builder, and building it ran the
first scan — tens of milliseconds of socket and process tables. The app
appeared and then stopped responding on its own first frame. It scans before
the terminal comes up now. The framework has no answer for slow work on the
frame thread in general; this app just stopped doing it there.

### 15. Slow work had nowhere to go

Every real terminal app reads files, runs subprocesses, and waits on networks.
Doing any of it between two frames stops the app: input queues, animations
stall, the screen holds still. The only offer was `sigil_tasks::spawn`, which
wants an async runtime — more than a binary needing "re-scan every few seconds
without freezing" should have to stand up.

`sigil::work(job)` runs a job on a worker thread and hands back a signal that
fills in when it finishes. The frame loop collects finished jobs on the tick
it already uses for animations and timers, so a result lands on a frame
boundary and the tree rebuilds from it like any other signal change. A job
that panics reports the panic rather than waiting at `Pending` forever.

This app now scans on a worker. The scanner sits behind a lock rather than
riding through the signal, because it holds a `System` that is not `Clone` —
the rows travel, the reader stays.

## Still open

- **`.empty()` takes `NodeBuilder` while `.cell()` takes `impl IntoNode`.**
  Sibling slots on one builder with different bounds, so the caller wraps a
  label in a container for no reason the signature explains. Left alone on
  purpose: `empty` hands its result straight to `children_reactive`, which
  wants a `NodeBuilder`, so widening the bound means wrapping every
  caller's node in a container to save one caller writing the wrapper. That
  trade is not worth a node in everybody's tree.
- **`TableState::new` panics outside a reactive scope.** State must be
  built inside the `TreeRoot` callback, so it cannot be constructed once
  and shared with a caller outside the tree. The panic names the file that
  needs the scope, which made it a two-minute fix rather than a long one.
- **Borders do not draw.** The resolved edge colour reaches `Painted` and
  the box-drawing characters are two lines of code, but a one-pixel border
  on an eight-pixel grid does not inset children by a whole cell, so the
  edge and the text beside it land in the same column: drawing it ate the
  first character of every header. That wants a layout inset, not a paint
  change.
- **Style resolves from scratch every frame.** The window host caches on
  node version; the cell host does not. Version alone is not enough — an
  animator updates values without bumping it — so the key has to account
  for that.
- **Two table tests fail, and did before this branch.**
  `crates/sigil/tests/table_view.rs` `wheel_scroll_advances_the_window` and
  `quick_filter_keeps_keyboard_row_count_live` panic with
  `layout: NodeMissing`. Both reproduce at `73ee6fe`, so they are older than
  this work, but they are real: after a scroll or filter rebuilds the window,
  the tree holds a child id that is no longer in the arena. That is
  `virtual_list`'s keyed rebuild leaving a dangling reference, not a test
  problem. Everything else in the workspace passes — 1524 tests across 231
  targets, once the `bench-parity-gpui` crate is excluded (its `cbindgen`
  build-dependency does not build on this machine).
- **No grapheme clustering.** Text is placed by display width, so a CJK glyph
  keeps its two columns, but a combining mark still takes a cell of its own
  and is overwritten by whatever follows it in the same run. Fixing it means a
  cell that can hold more than one `char`, which is the hottest data structure
  in the frame — worth measuring before committing to it.
- **Layout lands boxes off the cell grid.** The table above sits at
  `y=20.030`. Nothing is visibly wrong now that glyphs snap, but every
  fractional origin is a rounding decision waiting to go the other way. A
  cell host that snapped laid-out boxes to the grid would remove the
  question; that is a layout change, not a paint one.

## What the app is for

A development session starts a server and ends. The port stays
held. This machine had two duplicate Vite servers in one repo and a
ten-hour pair in another when the example was written.

A row is a stray when all four hold: it runs inside a dev root, it holds a
listening port, no terminal is attached, and it has outlived the limit.
Dropping any one of the four matches every system daemon on the machine —
the first kill list included ControlCenter, Discord, and `rapportd`.

The workspace `unsafe` denial is worth recording as a win. The first draft
called `libc::killpg`, `getpgid`, and `getpgrp`. Replacing them with one
`ps -axo pid=,pgid=,tty=` parse and `kill -TERM -<pgid>` dropped the
`libc` dependency and collapsed two per-process shell-outs into one call
for the whole table.
