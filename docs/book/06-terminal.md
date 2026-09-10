# The terminal host

This host paints the same widget tree onto a character grid. Swap
`launch` for `run_tui`. It is not a terminal emulator pane, not a
PTY, and not libghostty.

The same tree that runs in a window runs in a terminal. Swap the
entrypoint and keep everything else:

```rust
use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run_tui("My app", |cx: &mut Cx| {
        let count = signal(0_i32);
        cx.add(
            container()
                .child(text(derived(move || format!("Count: {}", count.get()))))
                .child(button("Increment").activate(move || count.set(count.peek() + 1))),
        )
    })
}
```

Build without the GPU stack by disabling default features:

```toml
sigil-ui = { git = "https://github.com/ThomGardiner/sigil.git", tag = "v0.1.0", default-features = false, features = ["tui"] }
```

Layout runs the same engine at `Host::CELL` (8 px) per terminal cell,
so widget proportions match the window host. A pointer report maps to
the centre of that cell, not its corner. Paint rasterizes onto a
character grid and writes only the rows that changed.

## What reaches the terminal

| Channel | Terminal host |
| --- | --- |
| Text, wrapping, ellipsis | Yes |
| Foreground and background color | Yes — resolved theme colors as 24-bit SGR when supported, otherwise the 256-color cube |
| Bold (weight ≥ 700) | Yes |
| Progress fills, chart bars | Yes — block glyphs |
| Borders, corner radius, shadows, opacity | No — a cell has no sub-character geometry |
| Images (`image_rgba`, `image_bytes`) | No — the node lays out, nothing paints |
| Animations | Yes — the frame clock ticks at ~30 Hz |

## Input

| Channel | Terminal host |
| --- | --- |
| Typed characters (including shift) | Yes — delivered as key events with `text` |
| Control and alt chords | Yes — as commands, never as text |
| Paste | Yes — bracketed paste arrives whole through the text channel |
| Mouse: click, hover, wheel, drag | Yes |
| Key release (`KeyPhase::Up`) | No — terminals report presses only; handlers keyed on release never fire |
| IME preedit | No — committed text arrives, composition UI does not |
| Tab / Shift-Tab focus ring | Yes |

`Ctrl-C` always exits. `Esc` exits when the config enables it.

## Off-the-frame work

`sigil::work(job)` runs a closure on a background thread and hands back
a `Signal<Status<T, String>>` the tree can render; finished jobs land
on the frame clock. `sigil::every(period, f)` schedules periodic work
on the same clock. `Screen` holds the buffers when a host runs its own
loop.

## Testing

`sigil::present(tree, cols, rows, &events)` (feature `tui`) mounts,
routes, and returns the painted grid as text — assert on what a reader
sees. `sigil::step` drives one frame at a time when a test owns the
host and router. Build pointer events with `sigil_tui::click(col, row)`
or `pointer(phase, col, row)` next to `key_down`. See
`crates/tui/tests/host.rs`.
